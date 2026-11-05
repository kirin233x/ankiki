//! 导入 Anki 词书（.apkg / .colpkg）。
//!
//! 支持三种内部格式：
//! - `collection.anki2`   （Anki 2.0 旧格式）
//! - `collection.anki21`  （Anki 2.1 旧格式）
//! - `collection.anki21b` （Anki 2.1.50+ 新格式：zstd 压缩 + protobuf 元数据）

use crate::fsrs;
use crate::models::*;
use crate::util::{self, now_secs, safe_filename, strip_html};
use crate::Collection;
use anyhow::{anyhow, Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};

const ZSTD_MAGIC: [u8; 4] = [0x28, 0xB5, 0x2F, 0xFD];

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOptions {
    /// 是否保留词书里的学习进度（间隔、到期时间等）
    #[serde(default = "default_true")]
    pub include_progress: bool,
    /// 自定义词书名称，留空则自动取名
    #[serde(default)]
    pub deck_name: Option<String>,
}

fn default_true() -> bool {
    true
}

impl Default for ImportOptions {
    fn default() -> Self {
        Self {
            include_progress: true,
            deck_name: None,
        }
    }
}

pub(crate) struct AnkiNotetype {
    name: String,
    kind: i64,
    css: String,
    sort_idx: i64,
    fields: Vec<String>,
    templates: Vec<TemplateDef>,
}

pub(crate) struct AnkiRevlog {
    cid: i64,
    ts_ms: i64,
    ease: i64,
    ivl: i64,
    rtype: i64,
    time_ms: i64,
}

pub(crate) struct AnkiNote {
    id: i64,
    guid: String,
    mid: i64,
    fields: Vec<String>,
    tags: String,
}

pub(crate) struct AnkiCard {
    id: i64,
    nid: i64,
    did: i64,
    ord: i64,
    ctype: i64,
    queue: i64,
    due: i64,
    ivl: i64,
    factor: i64,
    reps: i64,
    lapses: i64,
    data: String,
}

pub(crate) struct AnkiCollection {
    crt: i64,
    revlog: Vec<AnkiRevlog>,
    notetypes: HashMap<i64, AnkiNotetype>,
    decks: HashMap<i64, String>,
    pub(crate) notes: Vec<AnkiNote>,
    cards: Vec<AnkiCard>,
}

/// 删除临时文件的守卫
pub(crate) struct TempFile(pub PathBuf);
impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
        for ext in ["-wal", "-shm", "-journal"] {
            let mut p = self.0.clone().into_os_string();
            p.push(ext);
            let _ = std::fs::remove_file(PathBuf::from(p));
        }
    }
}

impl Collection {
    pub fn import_package(
        &mut self,
        path: &Path,
        opts: &ImportOptions,
        progress: &dyn Fn(ImportProgress),
    ) -> Result<ImportResult> {
        let report = |stage: &str, current: u64, total: u64| {
            progress(ImportProgress {
                stage: stage.to_string(),
                current,
                total,
            })
        };
        report("正在读取词书…", 0, 0);
        let file = File::open(path).with_context(|| format!("无法打开文件 {}", path.display()))?;
        let mut zip = zip::ZipArchive::new(BufReader::new(file))
            .context("文件不是有效的 Anki 词书（.apkg/.colpkg 应为 zip 格式）")?;

        let names: HashSet<String> = zip.file_names().map(|s| s.to_string()).collect();
        let coll_name = ["collection.anki21b", "collection.anki21", "collection.anki2"]
            .into_iter()
            .find(|n| names.contains(*n))
            .ok_or_else(|| anyhow!("词书中没有找到卡片数据（collection.anki2*）"))?;

        // 解出数据库到临时文件
        let tmp = TempFile(std::env::temp_dir().join(format!(
            "ankiki-import-{}-{}-{:x}.db",
            std::process::id(),
            util::now_millis(),
            rand::random::<u64>()
        )));
        {
            let entry = zip.by_name(coll_name)?;
            let mut out = File::create(&tmp.0)?;
            copy_maybe_zstd(entry, &mut out)?;
            out.flush()?;
        }
        let anki = read_collection(&tmp.0)?;
        if anki.notes.is_empty() {
            return Err(anyhow!("词书里没有任何单词"));
        }

        // 读取媒体清单
        let media_map = match zip.by_name("media") {
            Ok(mut entry) => {
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf)?;
                parse_media_map(&buf).unwrap_or_default()
            }
            Err(_) => Vec::new(),
        };

        // 词书名称
        let deck_name = opts
            .deck_name
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| guess_deck_name(&anki, path));

        let mut result = self.write_import(&anki, &deck_name, opts, &report)?;

        // 复制媒体文件
        let media_dir_name = format!(
            "deck{}-{}-{:x}",
            result.deck_id,
            util::now_millis(),
            rand::random::<u16>()
        );
        let media_dir = self.media_root.join(&media_dir_name);
        if !media_map.is_empty() {
            std::fs::create_dir_all(&media_dir)?;
        }
        let total = media_map.len() as u64;
        for (i, (zip_name, file_name)) in media_map.iter().enumerate() {
            if i % 20 == 0 {
                report("正在复制音频和图片…", i as u64, total);
            }
            let Some(safe) = safe_filename(file_name) else { continue };
            let Ok(entry) = zip.by_name(zip_name) else { continue };
            let mut out = File::create(media_dir.join(&safe))?;
            if copy_maybe_zstd(entry, &mut out).is_ok() {
                result.media_files += 1;
            }
        }
        self.conn.execute(
            "UPDATE notetypes SET media_dir = ? WHERE id IN
               (SELECT DISTINCT notetype_id FROM notes WHERE deck_id = ?) AND id != ?",
            params![media_dir_name, result.deck_id, crate::BASIC_NOTETYPE_ID],
        )?;
        report("导入完成", total, total);
        Ok(result)
    }

    fn write_import(
        &mut self,
        anki: &AnkiCollection,
        deck_name: &str,
        opts: &ImportOptions,
        report: &dyn Fn(&str, u64, u64),
    ) -> Result<ImportResult> {
        let now = now_secs();
        let existing: HashSet<String> = {
            let mut stmt = self.conn.prepare("SELECT guid FROM notes")?;
            let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
            rows.collect::<std::result::Result<_, _>>()?
        };

        let tx = self.conn.unchecked_transaction()?;
        tx.execute(
            "INSERT INTO decks (name, created_at, config) VALUES (?, ?, ?)",
            params![
                deck_name,
                now,
                serde_json::to_string(&DeckConfig::default())?
            ],
        )?;
        let deck_id = tx.last_insert_rowid();
        let mut result = ImportResult {
            deck_id,
            deck_name: deck_name.to_string(),
            ..Default::default()
        };

        // 笔记类型
        let mut nt_map: HashMap<i64, (i64, i64)> = HashMap::new(); // anki mid -> (our id, sort_idx)
        let used_mids: HashSet<i64> = anki.notes.iter().map(|n| n.mid).collect();
        for mid in used_mids {
            let Some(nt) = anki.notetypes.get(&mid) else { continue };
            tx.execute(
                "INSERT INTO notetypes (name, kind, css, fields, templates, sort_idx, media_dir)
                 VALUES (?, ?, ?, ?, ?, ?, '')",
                params![
                    nt.name,
                    nt.kind,
                    nt.css,
                    serde_json::to_string(&nt.fields)?,
                    serde_json::to_string(&nt.templates)?,
                    nt.sort_idx
                ],
            )?;
            nt_map.insert(mid, (tx.last_insert_rowid(), nt.sort_idx));
        }

        // 笔记
        let mut note_map: HashMap<i64, i64> = HashMap::new(); // anki nid -> our id
        let total = anki.notes.len() as u64;
        {
            let mut ins = tx.prepare(
                "INSERT INTO notes (guid, deck_id, notetype_id, fields, tags, sort_field, created_at, modified_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )?;
            let mut seen = HashSet::new();
            for (i, n) in anki.notes.iter().enumerate() {
                if i % 500 == 0 {
                    report("正在导入单词…", i as u64, total);
                }
                let Some(&(ntid, sort_idx)) = nt_map.get(&n.mid) else { continue };
                if existing.contains(&n.guid) || !seen.insert(n.guid.clone()) {
                    result.duplicates += 1;
                    continue;
                }
                let sort_field = n
                    .fields
                    .get(sort_idx as usize)
                    .or_else(|| n.fields.first())
                    .map(|f| strip_html(f))
                    .unwrap_or_default();
                let created = if n.id > 1_000_000_000_000 { n.id / 1000 } else { now };
                ins.execute(params![
                    n.guid,
                    deck_id,
                    ntid,
                    serde_json::to_string(&n.fields)?,
                    n.tags.trim(),
                    sort_field,
                    created,
                    now
                ])?;
                note_map.insert(n.id, tx.last_insert_rowid());
                result.notes_added += 1;
            }
        }

        // 卡片：新卡按原词书顺序排列
        let mut cards: Vec<&AnkiCard> = anki
            .cards
            .iter()
            .filter(|c| note_map.contains_key(&c.nid))
            .collect();
        cards.sort_by_key(|c| {
            let is_new = c.ctype == 0 || !opts.include_progress;
            (if is_new { 0 } else { 1 }, if c.ctype == 0 { c.due } else { 0 }, c.nid, c.ord)
        });
        report("正在生成卡片…", 0, cards.len() as u64);
        let last_rev = last_review_map(anki);
        let mut card_map: HashMap<i64, (i64, i64)> = HashMap::new(); // anki cid -> (our id, deck)
        {
            let mut ins = tx.prepare(
                "INSERT INTO cards (note_id, deck_id, ord, state, due, interval, step, stability,
                    difficulty, reps, lapses, last_review, suspended, position, created_at)
                 VALUES (?, ?, ?, ?, ?, ?, 0, ?, ?, ?, ?, ?, ?, ?, ?)",
            )?;
            for (pos, c) in cards.iter().enumerate() {
                let note_id = note_map[&c.nid];
                let p = convert_progress(
                    c,
                    anki.crt,
                    now,
                    opts.include_progress,
                    last_rev.get(&c.id).copied(),
                );
                if p.state != STATE_NEW {
                    result.with_progress += 1;
                }
                ins.execute(params![
                    note_id,
                    deck_id,
                    c.ord,
                    p.state,
                    p.due,
                    p.interval,
                    p.stability,
                    p.difficulty,
                    p.reps,
                    p.lapses,
                    p.last_review,
                    (c.queue == -1) as i64,
                    pos as i64,
                    now
                ])?;
                card_map.insert(c.id, (tx.last_insert_rowid(), deck_id));
                result.cards_added += 1;
            }
        }
        if opts.include_progress {
            insert_revlog(&tx, anki, &card_map)?;
        }
        tx.commit()?;

        // 已经导入过的单词：用词书里的进度更新（取较新的一方）
        if opts.include_progress && result.duplicates > 0 {
            report("正在同步已有单词的学习进度…", 0, 0);
            let up = self.apply_anki_progress(anki)?;
            result.progress_updated = up.updated_cards;
            if result.notes_added == 0 {
                if let Some(d) = up.first_deck {
                    result.deck_id = d;
                    result.deck_name = self.deck_name(d)?;
                }
            }
        }

        if result.notes_added == 0 {
            // 全部重复：删除空词书
            self.delete_deck(deck_id)?;
            if result.progress_updated > 0 {
                return Ok(result);
            }
            return Err(anyhow!(
                "这本词书里的 {} 个单词已经全部导入过了，也没有更新的学习进度",
                result.duplicates
            ));
        }
        Ok(result)
    }
}

struct Progress {
    state: i64,
    due: i64,
    interval: i64,
    stability: f64,
    difficulty: f64,
    reps: i64,
    lapses: i64,
    last_review: i64,
}

fn convert_progress(c: &AnkiCard, crt: i64, now: i64, include: bool, last_rev: Option<i64>) -> Progress {
    let fresh = Progress {
        state: STATE_NEW,
        due: 0,
        interval: 0,
        stability: 0.0,
        difficulty: 0.0,
        reps: 0,
        lapses: 0,
        last_review: 0,
    };
    if !include || c.ctype == 0 {
        return fresh;
    }
    let data: Value = serde_json::from_str(&c.data).unwrap_or(Value::Null);
    let fsrs_s = data.get("s").and_then(Value::as_f64).filter(|v| *v > 0.0);
    let fsrs_d = data.get("d").and_then(Value::as_f64).filter(|v| *v > 0.0);
    let lrt = data.get("lrt").and_then(Value::as_i64).filter(|v| *v > 0);

    let ivl = c.ivl.max(1);
    let (state, due) = match c.ctype {
        2 => {
            // 复习卡的 due 是相对词书创建日的天数
            let due = if crt > 0 { crt + c.due * 86400 } else { now };
            (STATE_REVIEW, due)
        }
        3 => (STATE_RELEARNING, now),
        _ => (STATE_LEARNING, now),
    };
    let stability = fsrs_s.unwrap_or(if c.ctype == 1 { 0.0 } else { ivl as f64 });
    let difficulty = fsrs_d
        .map(|d| d.clamp(1.0, 10.0))
        .unwrap_or_else(|| fsrs::difficulty_from_ease(c.factor));
    let last_review = lrt.or(last_rev).unwrap_or(if state == STATE_REVIEW {
        due - ivl * 86400
    } else {
        now
    });
    Progress {
        state,
        due,
        interval: if c.ctype == 1 { 0 } else { ivl },
        stability,
        difficulty: if stability > 0.0 { difficulty } else { 0.0 },
        reps: c.reps,
        lapses: c.lapses,
        last_review: last_review.min(now),
    }
}

fn guess_deck_name(anki: &AnkiCollection, path: &Path) -> String {
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "导入的词书".into());
    let mut roots: HashSet<String> = HashSet::new();
    for c in &anki.cards {
        if let Some(name) = anki.decks.get(&c.did) {
            let root = name.split("::").next().unwrap_or(name).trim().to_string();
            roots.insert(root);
        }
    }
    if roots.len() == 1 {
        let root = roots.into_iter().next().unwrap();
        if !root.is_empty() && root != "Default" && root != "默认" {
            return root;
        }
    }
    stem
}

/// 复制 zip 条目，如果是 zstd 压缩则自动解压
fn copy_maybe_zstd<R: Read>(reader: R, out: &mut impl Write) -> Result<u64> {
    let mut br = BufReader::with_capacity(64 * 1024, reader);
    let is_zstd = br.fill_buf()?.starts_with(&ZSTD_MAGIC);
    if is_zstd {
        let mut dec = zstd::stream::read::Decoder::with_buffer(br)?;
        Ok(std::io::copy(&mut dec, out)?)
    } else {
        Ok(std::io::copy(&mut br, out)?)
    }
}

/// 解析媒体清单，返回 (zip 内文件名, 原始文件名)
fn parse_media_map(raw: &[u8]) -> Result<Vec<(String, String)>> {
    let data = if raw.starts_with(&ZSTD_MAGIC) {
        zstd::stream::decode_all(raw)?
    } else {
        raw.to_vec()
    };
    let trimmed = data
        .iter()
        .position(|b| !b.is_ascii_whitespace())
        .map(|i| &data[i..])
        .unwrap_or(&[]);
    if trimmed.first() == Some(&b'{') {
        let map: HashMap<String, String> = serde_json::from_slice(trimmed)?;
        let mut v: Vec<_> = map.into_iter().collect();
        v.sort_by_key(|(k, _)| k.parse::<u64>().unwrap_or(u64::MAX));
        return Ok(v);
    }
    // 新格式：MediaEntries { repeated MediaEntry entries = 1; }
    // MediaEntry { string name = 1; uint32 size = 2; bytes sha1 = 3; optional uint32 legacy_zip_filename = 255; }
    let mut out = Vec::new();
    for (field, value) in pb::fields(&data)? {
        if field != 1 {
            continue;
        }
        let pb::Value::Bytes(entry) = value else { continue };
        let mut name = String::new();
        let mut legacy: Option<u64> = None;
        for (f, v) in pb::fields(entry)? {
            match (f, v) {
                (1, pb::Value::Bytes(b)) => name = String::from_utf8_lossy(b).into_owned(),
                (255, pb::Value::Varint(n)) => legacy = Some(n),
                _ => {}
            }
        }
        let zip_name = legacy
            .map(|n| n.to_string())
            .unwrap_or_else(|| out.len().to_string());
        out.push((zip_name, name));
    }
    Ok(out)
}

pub(crate) fn read_collection(path: &Path) -> Result<AnkiCollection> {
    let conn = Connection::open(path).context("无法读取词书数据库")?;
    // 新版 Anki 的表使用了自定义的 unicase 排序规则
    conn.create_collation("unicase", |a: &str, b: &str| {
        a.to_lowercase().cmp(&b.to_lowercase())
    })?;

    let has_table = |name: &str| -> Result<bool> {
        Ok(conn
            .query_row(
                "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?",
                [name],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false))
    };

    let crt: i64 = conn
        .query_row("SELECT crt FROM col LIMIT 1", [], |r| r.get(0))
        .unwrap_or(0);

    let (notetypes, decks) = if has_table("notetypes")? {
        (read_notetypes_v18(&conn)?, read_decks_v18(&conn)?)
    } else {
        let (models, decks): (String, String) =
            conn.query_row("SELECT models, decks FROM col LIMIT 1", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })?;
        (parse_legacy_models(&models)?, parse_legacy_decks(&decks))
    };

    let notes = {
        let mut stmt = conn.prepare("SELECT id, guid, mid, flds, tags FROM notes ORDER BY id")?;
        let rows = stmt.query_map([], |r| {
            let flds: String = r.get(3)?;
            Ok(AnkiNote {
                id: r.get(0)?,
                guid: r.get(1)?,
                mid: r.get(2)?,
                fields: flds.split('\u{1f}').map(|s| s.to_string()).collect(),
                tags: r.get(4)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()?
    };

    let has_data = conn
        .prepare("SELECT data FROM cards LIMIT 0")
        .is_ok();
    let sql = format!(
        "SELECT nid, did, odid, ord, type, queue, due, odue, ivl, factor, reps, lapses, {}, id FROM cards ORDER BY id",
        if has_data { "data" } else { "''" }
    );
    let cards = {
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], |r| {
            let did: i64 = r.get(1)?;
            let odid: i64 = r.get(2)?;
            let due: i64 = r.get(6)?;
            let odue: i64 = r.get(7)?;
            // 在筛选卡组中的卡片，使用原卡组和原到期时间
            let (did, due) = if odid != 0 {
                (odid, if odue != 0 { odue } else { due })
            } else {
                (did, due)
            };
            Ok(AnkiCard {
                id: r.get(13)?,
                nid: r.get(0)?,
                did,
                ord: r.get(3)?,
                ctype: r.get(4)?,
                queue: r.get(5)?,
                due,
                ivl: r.get(8)?,
                factor: r.get(9)?,
                reps: r.get(10)?,
                lapses: r.get(11)?,
                data: r.get::<_, Option<String>>(12)?.unwrap_or_default(),
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()?
    };

    // 复习记录（ease 为 0 的是手动调整，跳过）
    let revlog = if has_table("revlog")? {
        let mut stmt = conn.prepare(
            "SELECT id, cid, ease, ivl, type, time FROM revlog WHERE ease BETWEEN 1 AND 4 ORDER BY id",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok(AnkiRevlog {
                ts_ms: r.get(0)?,
                cid: r.get(1)?,
                ease: r.get(2)?,
                ivl: r.get(3)?,
                rtype: r.get(4)?,
                time_ms: r.get(5)?,
            })
        })?;
        rows.collect::<std::result::Result<Vec<_>, _>>()?
    } else {
        Vec::new()
    };

    Ok(AnkiCollection {
        crt,
        revlog,
        notetypes,
        decks,
        notes,
        cards,
    })
}

fn parse_legacy_models(raw: &str) -> Result<HashMap<i64, AnkiNotetype>> {
    let v: Value = serde_json::from_str(raw).context("无法解析笔记类型")?;
    let mut out = HashMap::new();
    let Some(obj) = v.as_object() else { return Ok(out) };
    for (key, m) in obj {
        let id = m
            .get("id")
            .and_then(Value::as_i64)
            .or_else(|| key.parse().ok())
            .unwrap_or(0);
        let mut flds: Vec<(i64, String)> = m
            .get("flds")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .enumerate()
                    .map(|(i, f)| {
                        (
                            f.get("ord").and_then(Value::as_i64).unwrap_or(i as i64),
                            f.get("name").and_then(Value::as_str).unwrap_or("").to_string(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        flds.sort_by_key(|f| f.0);
        let mut tmpls: Vec<(i64, TemplateDef)> = m
            .get("tmpls")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .enumerate()
                    .map(|(i, t)| {
                        let s = |k: &str| t.get(k).and_then(Value::as_str).unwrap_or("").to_string();
                        (
                            t.get("ord").and_then(Value::as_i64).unwrap_or(i as i64),
                            TemplateDef {
                                name: s("name"),
                                qfmt: s("qfmt"),
                                afmt: s("afmt"),
                            },
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        tmpls.sort_by_key(|t| t.0);
        out.insert(
            id,
            AnkiNotetype {
                name: m.get("name").and_then(Value::as_str).unwrap_or("").to_string(),
                kind: m.get("type").and_then(Value::as_i64).unwrap_or(0),
                css: m.get("css").and_then(Value::as_str).unwrap_or("").to_string(),
                sort_idx: m.get("sortf").and_then(Value::as_i64).unwrap_or(0),
                fields: flds.into_iter().map(|f| f.1).collect(),
                templates: tmpls.into_iter().map(|t| t.1).collect(),
            },
        );
    }
    Ok(out)
}

fn parse_legacy_decks(raw: &str) -> HashMap<i64, String> {
    let v: Value = serde_json::from_str(raw).unwrap_or(Value::Null);
    let mut out = HashMap::new();
    if let Some(obj) = v.as_object() {
        for (key, d) in obj {
            let id = d
                .get("id")
                .and_then(Value::as_i64)
                .or_else(|| key.parse().ok())
                .unwrap_or(0);
            let name = d.get("name").and_then(Value::as_str).unwrap_or("").to_string();
            out.insert(id, name);
        }
    }
    out
}

fn read_notetypes_v18(conn: &Connection) -> Result<HashMap<i64, AnkiNotetype>> {
    let mut out = HashMap::new();
    {
        let mut stmt = conn.prepare("SELECT id, name, config FROM notetypes")?;
        let mut rows = stmt.query([])?;
        while let Some(r) = rows.next()? {
            let id: i64 = r.get(0)?;
            let name: String = r.get(1)?;
            let config: Vec<u8> = r.get::<_, Option<Vec<u8>>>(2)?.unwrap_or_default();
            // Notetype.Config { kind = 1; sort_field_idx = 2; css = 3; ... }
            let mut nt = AnkiNotetype {
                name,
                kind: 0,
                css: String::new(),
                sort_idx: 0,
                fields: vec![],
                templates: vec![],
            };
            for (f, v) in pb::fields(&config).unwrap_or_default() {
                match (f, v) {
                    (1, pb::Value::Varint(k)) => nt.kind = k as i64,
                    (2, pb::Value::Varint(s)) => nt.sort_idx = s as i64,
                    (3, pb::Value::Bytes(b)) => nt.css = String::from_utf8_lossy(b).into_owned(),
                    _ => {}
                }
            }
            out.insert(id, nt);
        }
    }
    {
        let mut stmt = conn.prepare("SELECT ntid, ord, name FROM fields")?;
        let mut rows = stmt.query([])?;
        let mut tmp: HashMap<i64, Vec<(i64, String)>> = HashMap::new();
        while let Some(r) = rows.next()? {
            tmp.entry(r.get(0)?)
                .or_default()
                .push((r.get(1)?, r.get(2)?));
        }
        for (ntid, mut v) in tmp {
            v.sort_by_key(|x| x.0);
            if let Some(nt) = out.get_mut(&ntid) {
                nt.fields = v.into_iter().map(|x| x.1).collect();
            }
        }
    }
    {
        let mut stmt = conn.prepare("SELECT ntid, ord, name, config FROM templates")?;
        let mut rows = stmt.query([])?;
        let mut tmp: HashMap<i64, Vec<(i64, TemplateDef)>> = HashMap::new();
        while let Some(r) = rows.next()? {
            let config: Vec<u8> = r.get::<_, Option<Vec<u8>>>(3)?.unwrap_or_default();
            // Template.Config { q_format = 1; a_format = 2; ... }
            let mut t = TemplateDef {
                name: r.get(2)?,
                qfmt: String::new(),
                afmt: String::new(),
            };
            for (f, v) in pb::fields(&config).unwrap_or_default() {
                match (f, v) {
                    (1, pb::Value::Bytes(b)) => t.qfmt = String::from_utf8_lossy(b).into_owned(),
                    (2, pb::Value::Bytes(b)) => t.afmt = String::from_utf8_lossy(b).into_owned(),
                    _ => {}
                }
            }
            tmp.entry(r.get(0)?).or_default().push((r.get(1)?, t));
        }
        for (ntid, mut v) in tmp {
            v.sort_by_key(|x| x.0);
            if let Some(nt) = out.get_mut(&ntid) {
                nt.templates = v.into_iter().map(|x| x.1).collect();
            }
        }
    }
    Ok(out)
}

fn read_decks_v18(conn: &Connection) -> Result<HashMap<i64, String>> {
    let mut stmt = conn.prepare("SELECT id, name FROM decks")?;
    let rows = stmt.query_map([], |r| {
        let name: String = r.get(1)?;
        Ok((r.get::<_, i64>(0)?, name.replace('\u{1f}', "::")))
    })?;
    Ok(rows.collect::<std::result::Result<_, _>>()?)
}

/// 极简 protobuf 解码器，只需要读取少量字段
mod pb {
    use anyhow::{anyhow, Result};

    #[derive(Debug, Clone, Copy)]
    pub enum Value<'a> {
        Varint(u64),
        Bytes(&'a [u8]),
        Fixed64,
        Fixed32,
    }

    fn varint(buf: &[u8], pos: &mut usize) -> Result<u64> {
        let mut result = 0u64;
        let mut shift = 0;
        loop {
            let b = *buf.get(*pos).ok_or_else(|| anyhow!("protobuf 数据被截断"))?;
            *pos += 1;
            result |= ((b & 0x7f) as u64) << shift;
            if b & 0x80 == 0 {
                return Ok(result);
            }
            shift += 7;
            if shift >= 64 {
                return Err(anyhow!("protobuf varint 过长"));
            }
        }
    }

    pub fn fields(buf: &[u8]) -> Result<Vec<(u32, Value<'_>)>> {
        let mut pos = 0;
        let mut out = Vec::new();
        while pos < buf.len() {
            let key = varint(buf, &mut pos)?;
            let field = (key >> 3) as u32;
            let value = match key & 7 {
                0 => Value::Varint(varint(buf, &mut pos)?),
                1 => {
                    let b = buf
                        .get(pos..pos + 8)
                        .ok_or_else(|| anyhow!("protobuf 数据被截断"))?;
                    pos += 8;
                    let _ = b;
                    Value::Fixed64
                }
                2 => {
                    let len = varint(buf, &mut pos)? as usize;
                    let b = buf
                        .get(pos..pos + len)
                        .ok_or_else(|| anyhow!("protobuf 数据被截断"))?;
                    pos += len;
                    Value::Bytes(b)
                }
                5 => {
                    let b = buf
                        .get(pos..pos + 4)
                        .ok_or_else(|| anyhow!("protobuf 数据被截断"))?;
                    pos += 4;
                    let _ = b;
                    Value::Fixed32
                }
                t => return Err(anyhow!("不支持的 protobuf 类型 {t}")),
            };
            out.push((field, value));
        }
        Ok(out)
    }
}

/// 每张 Anki 卡片最后一次复习的时间（秒）
pub(crate) fn last_review_map(anki: &AnkiCollection) -> HashMap<i64, i64> {
    let mut m = HashMap::new();
    for r in &anki.revlog {
        let e = m.entry(r.cid).or_insert(0);
        *e = (*e).max(r.ts_ms / 1000);
    }
    m
}

/// 把 Anki 复习记录写入 revlog（按 卡片+时间 去重）。返回新增条数
pub(crate) fn insert_revlog(
    conn: &Connection,
    anki: &AnkiCollection,
    card_map: &HashMap<i64, (i64, i64)>,
) -> Result<i64> {
    let mut existing: HashMap<i64, HashSet<i64>> = HashMap::new();
    let mut first_seen: HashSet<i64> = HashSet::new();
    let mut added = 0;
    let mut sel = conn.prepare("SELECT ts FROM revlog WHERE card_id = ?")?;
    let mut ins = conn.prepare(
        "INSERT INTO revlog (card_id, deck_id, ts, rating, state, interval, duration_ms)
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )?;
    for r in &anki.revlog {
        let Some(&(card_id, deck_id)) = card_map.get(&r.cid) else { continue };
        let seen = match existing.entry(card_id) {
            std::collections::hash_map::Entry::Occupied(e) => e.into_mut(),
            std::collections::hash_map::Entry::Vacant(e) => {
                let rows = sel.query_map([card_id], |row| row.get::<_, i64>(0))?;
                e.insert(rows.collect::<std::result::Result<_, _>>()?)
            }
        };
        // 第一次作答记为「新卡」，便于统计每日新学
        let is_first = first_seen.insert(r.cid);
        if !seen.insert(r.ts_ms) {
            continue;
        }
        let state = match r.rtype {
            0 if is_first => STATE_NEW,
            0 => STATE_LEARNING,
            2 => STATE_RELEARNING,
            _ => STATE_REVIEW,
        };
        ins.execute(params![
            card_id,
            deck_id,
            r.ts_ms,
            r.ease,
            state,
            r.ivl,
            r.time_ms.clamp(0, 3_600_000)
        ])?;
        added += 1;
    }
    Ok(added)
}

#[derive(Debug, Default)]
pub(crate) struct ProgressUpdate {
    pub matched_notes: i64,
    pub updated_cards: i64,
    pub skipped_newer: i64,
    pub revlog_added: i64,
    pub missing_notes: i64,
    pub first_deck: Option<i64>,
}

impl Collection {
    /// 按 guid 匹配已有单词，用 Anki 中较新的学习进度覆盖 Ankiki 中的进度
    pub(crate) fn apply_anki_progress(&mut self, anki: &AnkiCollection) -> Result<ProgressUpdate> {
        let now = now_secs();
        let ours: HashMap<String, i64> = {
            let mut stmt = self.conn.prepare("SELECT guid, id FROM notes")?;
            let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?;
            rows.collect::<std::result::Result<_, _>>()?
        };
        let nid_guid: HashMap<i64, &str> = anki.notes.iter().map(|n| (n.id, n.guid.as_str())).collect();
        let last_rev = last_review_map(anki);
        let mut up = ProgressUpdate::default();
        let mut matched: HashSet<i64> = HashSet::new();
        let mut missing: HashSet<i64> = HashSet::new();
        let mut card_map: HashMap<i64, (i64, i64)> = HashMap::new();

        let tx = self.conn.unchecked_transaction()?;
        {
            let mut find = tx.prepare(
                "SELECT id, deck_id, state, last_review FROM cards WHERE note_id = ? AND ord = ?",
            )?;
            let mut update = tx.prepare(
                "UPDATE cards SET state = ?, due = ?, interval = ?, step = 0, stability = ?,
                    difficulty = ?, reps = ?, lapses = ?, last_review = ?,
                    suspended = CASE WHEN suspended = 2 THEN 2 ELSE ? END
                 WHERE id = ?",
            )?;
            for c in &anki.cards {
                let Some(guid) = nid_guid.get(&c.nid) else { continue };
                let Some(&note_id) = ours.get(*guid) else {
                    missing.insert(c.nid);
                    continue;
                };
                matched.insert(note_id);
                let Some((card_id, deck_id, state, last_review)) = find
                    .query_row(params![note_id, c.ord], |r| {
                        Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?, r.get::<_, i64>(3)?))
                    })
                    .optional()?
                else {
                    continue;
                };
                up.first_deck.get_or_insert(deck_id);
                if c.ctype == 0 {
                    // Anki 里还是新卡：不动 Ankiki 的进度
                    continue;
                }
                let p = convert_progress(c, anki.crt, now, true, last_rev.get(&c.id).copied());
                if state != STATE_NEW && last_review > p.last_review {
                    up.skipped_newer += 1;
                    continue;
                }
                update.execute(params![
                    p.state,
                    p.due,
                    p.interval,
                    p.stability,
                    p.difficulty,
                    p.reps,
                    p.lapses,
                    p.last_review,
                    (c.queue == -1) as i64,
                    card_id
                ])?;
                card_map.insert(c.id, (card_id, deck_id));
                up.updated_cards += 1;
            }
        }
        up.revlog_added = insert_revlog(&tx, anki, &card_map)?;
        tx.commit()?;
        self.undo = None;
        up.matched_notes = matched.len() as i64;
        up.missing_notes = missing.len() as i64;
        Ok(up)
    }
}
