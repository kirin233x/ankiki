//! Ankiki 核心库：与 UI 无关，可独立测试。

pub mod board;
pub mod browse;
pub mod fsrs;
pub mod importer;
pub mod models;
pub mod scheduler;
pub mod sift;
pub mod stats;
pub mod sync;
pub mod util;

use anyhow::{anyhow, Result};
use models::*;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};

pub use scheduler::Card;

pub const BASIC_NOTETYPE_ID: i64 = 1;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS decks (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    created_at  INTEGER NOT NULL,
    config      TEXT NOT NULL DEFAULT '{}'
);
CREATE TABLE IF NOT EXISTS notetypes (
    id        INTEGER PRIMARY KEY AUTOINCREMENT,
    name      TEXT NOT NULL,
    kind      INTEGER NOT NULL DEFAULT 0,
    css       TEXT NOT NULL DEFAULT '',
    fields    TEXT NOT NULL,
    templates TEXT NOT NULL,
    sort_idx  INTEGER NOT NULL DEFAULT 0,
    media_dir TEXT NOT NULL DEFAULT ''
);
CREATE TABLE IF NOT EXISTS notes (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    guid        TEXT NOT NULL,
    deck_id     INTEGER NOT NULL,
    notetype_id INTEGER NOT NULL,
    fields      TEXT NOT NULL,
    tags        TEXT NOT NULL DEFAULT '',
    sort_field  TEXT NOT NULL DEFAULT '',
    created_at  INTEGER NOT NULL,
    modified_at INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS notes_guid ON notes(guid);
CREATE INDEX IF NOT EXISTS notes_deck ON notes(deck_id);
CREATE TABLE IF NOT EXISTS cards (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    note_id     INTEGER NOT NULL,
    deck_id     INTEGER NOT NULL,
    ord         INTEGER NOT NULL DEFAULT 0,
    state       INTEGER NOT NULL DEFAULT 0,
    due         INTEGER NOT NULL DEFAULT 0,
    interval    INTEGER NOT NULL DEFAULT 0,
    step        INTEGER NOT NULL DEFAULT 0,
    stability   REAL NOT NULL DEFAULT 0,
    difficulty  REAL NOT NULL DEFAULT 0,
    reps        INTEGER NOT NULL DEFAULT 0,
    lapses      INTEGER NOT NULL DEFAULT 0,
    last_review INTEGER NOT NULL DEFAULT 0,
    suspended   INTEGER NOT NULL DEFAULT 0,
    position    INTEGER NOT NULL DEFAULT 0,
    created_at  INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS cards_deck_state ON cards(deck_id, state, due);
CREATE INDEX IF NOT EXISTS cards_note ON cards(note_id);
CREATE TABLE IF NOT EXISTS revlog (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    card_id     INTEGER NOT NULL,
    deck_id     INTEGER NOT NULL,
    ts          INTEGER NOT NULL,
    rating      INTEGER NOT NULL,
    state       INTEGER NOT NULL,
    interval    INTEGER NOT NULL,
    duration_ms INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS revlog_ts ON revlog(ts);
CREATE INDEX IF NOT EXISTS revlog_card ON revlog(card_id);
"#;

pub(crate) struct UndoEntry {
    pub card: Card,
    pub revlog_id: i64,
}

pub struct Collection {
    pub conn: Connection,
    pub media_root: PathBuf,
    pub(crate) undo: Option<UndoEntry>,
    settings: Settings,
}

impl Collection {
    /// 打开（或创建）位于 `data_dir` 的数据库
    pub fn open(data_dir: &Path) -> Result<Self> {
        std::fs::create_dir_all(data_dir)?;
        let media_root = data_dir.join("media");
        std::fs::create_dir_all(&media_root)?;
        let conn = Connection::open(data_dir.join("ankiki.db"))?;
        Self::from_connection(conn, media_root)
    }

    pub fn from_connection(conn: Connection, media_root: PathBuf) -> Result<Self> {
        conn.execute_batch(
            "PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA foreign_keys = OFF;",
        )?;
        conn.execute_batch(SCHEMA)?;
        let mut col = Self {
            conn,
            media_root,
            undo: None,
            settings: Settings::default(),
        };
        col.ensure_basic_notetype()?;
        col.settings = col.load_settings()?;
        Ok(col)
    }

    fn ensure_basic_notetype(&self) -> Result<()> {
        let exists: bool = self
            .conn
            .query_row(
                "SELECT 1 FROM notetypes WHERE id = ?",
                [BASIC_NOTETYPE_ID],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);
        if !exists {
            let templates = vec![TemplateDef {
                name: "卡片 1".into(),
                qfmt: "<div class=\"word\">{{正面}}</div>".into(),
                afmt: "{{FrontSide}}\n<hr id=\"answer\">\n<div class=\"meaning\">{{背面}}</div>"
                    .into(),
            }];
            self.conn.execute(
                "INSERT INTO notetypes (id, name, kind, css, fields, templates, sort_idx, media_dir)
                 VALUES (?, '基础', 0, ?, ?, ?, 0, '')",
                params![
                    BASIC_NOTETYPE_ID,
                    BASIC_CSS,
                    serde_json::to_string(&["正面", "背面"])?,
                    serde_json::to_string(&templates)?
                ],
            )?;
        }
        Ok(())
    }

    // ---------------------------------------------------------------- 设置

    fn load_settings(&self) -> Result<Settings> {
        let raw: Option<String> = self
            .conn
            .query_row("SELECT value FROM meta WHERE key = 'settings'", [], |r| {
                r.get(0)
            })
            .optional()?;
        Ok(raw
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default())
    }

    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    pub fn save_settings(&mut self, settings: Settings) -> Result<()> {
        let mut settings = settings;
        settings.rollover_hour = settings.rollover_hour.min(23);
        self.conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES ('settings', ?)",
            [serde_json::to_string(&settings)?],
        )?;
        self.settings = settings;
        Ok(())
    }

    pub(crate) fn rollover(&self) -> u32 {
        self.settings.rollover_hour
    }

    // ---------------------------------------------------------------- 卡组

    pub fn create_deck(&self, name: &str) -> Result<i64> {
        let name = name.trim();
        if name.is_empty() {
            return Err(anyhow!("词书名称不能为空"));
        }
        self.conn.execute(
            "INSERT INTO decks (name, created_at, config) VALUES (?, ?, ?)",
            params![
                name,
                util::now_secs(),
                serde_json::to_string(&DeckConfig::default())?
            ],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn rename_deck(&self, deck_id: i64, name: &str) -> Result<()> {
        let name = name.trim();
        if name.is_empty() {
            return Err(anyhow!("词书名称不能为空"));
        }
        self.conn
            .execute("UPDATE decks SET name = ? WHERE id = ?", params![name, deck_id])?;
        Ok(())
    }

    pub fn set_deck_description(&self, deck_id: i64, description: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE decks SET description = ? WHERE id = ?",
            params![description, deck_id],
        )?;
        Ok(())
    }

    pub fn deck_config(&self, deck_id: i64) -> Result<DeckConfig> {
        let raw: String = self
            .conn
            .query_row("SELECT config FROM decks WHERE id = ?", [deck_id], |r| {
                r.get(0)
            })
            .optional()?
            .ok_or_else(|| anyhow!("词书不存在"))?;
        Ok(parse_config(&raw))
    }

    pub fn update_deck_config(&self, deck_id: i64, config: DeckConfig) -> Result<()> {
        let mut c = config;
        c.new_per_day = c.new_per_day.clamp(0, 9999);
        c.reviews_per_day = c.reviews_per_day.clamp(0, 99999);
        c.desired_retention = c.desired_retention.clamp(0.7, 0.99);
        c.max_interval = c.max_interval.clamp(1, 36500);
        c.learning_steps.retain(|s| *s > 0.0);
        c.relearning_steps.retain(|s| *s > 0.0);
        let old = self.deck_config(deck_id)?;
        self.conn.execute(
            "UPDATE decks SET config = ? WHERE id = ?",
            params![serde_json::to_string(&c)?, deck_id],
        )?;
        if old.new_order != c.new_order {
            self.reorder_new_cards(deck_id, c.new_order == 1)?;
        }
        Ok(())
    }

    /// 重新排列新卡顺序（随机或按添加顺序）
    pub fn reorder_new_cards(&self, deck_id: i64, random: bool) -> Result<()> {
        let order = if random { "RANDOM()" } else { "note_id, ord" };
        let ids: Vec<i64> = {
            let mut stmt = self.conn.prepare(&format!(
                "SELECT id FROM cards WHERE deck_id = ? AND state = 0 AND position >= 0 ORDER BY {order}"
            ))?;
            let rows = stmt.query_map([deck_id], |r| r.get(0))?;
            rows.collect::<std::result::Result<_, _>>()?
        };
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut up = tx.prepare("UPDATE cards SET position = ? WHERE id = ?")?;
            for (i, id) in ids.iter().enumerate() {
                up.execute(params![i as i64, id])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn delete_deck(&mut self, deck_id: i64) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        tx.execute("DELETE FROM revlog WHERE deck_id = ?", [deck_id])?;
        tx.execute("DELETE FROM cards WHERE deck_id = ?", [deck_id])?;
        tx.execute("DELETE FROM notes WHERE deck_id = ?", [deck_id])?;
        tx.execute("DELETE FROM decks WHERE id = ?", [deck_id])?;
        tx.commit()?;
        self.undo = None;
        self.cleanup_notetypes()?;
        Ok(())
    }

    /// 删除不再被任何笔记使用的导入笔记类型及其媒体目录
    fn cleanup_notetypes(&self) -> Result<()> {
        let orphans: Vec<(i64, String)> = {
            let mut stmt = self.conn.prepare(
                "SELECT id, media_dir FROM notetypes
                 WHERE id != ? AND id NOT IN (SELECT DISTINCT notetype_id FROM notes)",
            )?;
            let rows = stmt.query_map([BASIC_NOTETYPE_ID], |r| Ok((r.get(0)?, r.get(1)?)))?;
            rows.collect::<std::result::Result<_, _>>()?
        };
        for (id, media_dir) in orphans {
            // 同一次导入的多个笔记类型共用一个媒体目录，只有都不再使用时才删除
            let still_used: i64 = self.conn.query_row(
                "SELECT COUNT(*) FROM notetypes WHERE media_dir = ? AND id != ?
                 AND id IN (SELECT DISTINCT notetype_id FROM notes)",
                params![media_dir, id],
                |r| r.get(0),
            )?;
            self.conn.execute("DELETE FROM notetypes WHERE id = ?", [id])?;
            if !media_dir.is_empty() && still_used == 0 {
                let path = self.media_root.join(&media_dir);
                if path.starts_with(&self.media_root) && path != self.media_root {
                    let _ = std::fs::remove_dir_all(path);
                }
            }
        }
        Ok(())
    }

    pub fn list_decks(&self) -> Result<Vec<DeckInfo>> {
        let decks: Vec<(i64, String, String, i64, String)> = {
            let mut stmt = self.conn.prepare(
                "SELECT id, name, description, created_at, config FROM decks ORDER BY created_at, id",
            )?;
            let rows = stmt.query_map([], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
            })?;
            rows.collect::<std::result::Result<_, _>>()?
        };
        let mut out = Vec::with_capacity(decks.len());
        for (id, name, description, created_at, config) in decks {
            let config = parse_config(&config);
            let due = self.deck_counts(id, &config)?;
            let (total, new_cards, mature, suspended, known): (i64, i64, i64, i64, i64) =
                self.conn.query_row(
                    "SELECT COUNT(*),
                        COALESCE(SUM(state = 0 AND suspended != 2), 0),
                        COALESCE(SUM(state = 2 AND interval >= 21 AND suspended != 2), 0),
                        COALESCE(SUM(suspended = 1), 0),
                        COALESCE(SUM(suspended = 2), 0)
                 FROM cards WHERE deck_id = ?",
                    [id],
                    |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?)),
                )?;
            let studied_today = self.studied_today(Some(id))?;
            out.push(DeckInfo {
                id,
                name,
                description,
                created_at,
                config,
                due,
                total_cards: total,
                new_cards,
                learned_cards: total - new_cards,
                mature_cards: mature,
                suspended_cards: suspended,
                known_cards: known,
                studied_today,
            });
        }
        Ok(out)
    }

    pub(crate) fn deck_name(&self, deck_id: i64) -> Result<String> {
        Ok(self
            .conn
            .query_row("SELECT name FROM decks WHERE id = ?", [deck_id], |r| r.get(0))
            .optional()?
            .unwrap_or_default())
    }

    // ---------------------------------------------------------------- 笔记

    pub fn notetype(&self, id: i64) -> Result<NotetypeView> {
        let (name, kind, css, fields, templates, media_dir): (
            String,
            i64,
            String,
            String,
            String,
            String,
        ) = self
            .conn
            .query_row(
                "SELECT name, kind, css, fields, templates, media_dir FROM notetypes WHERE id = ?",
                [id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
            )
            .optional()?
            .ok_or_else(|| anyhow!("笔记类型不存在"))?;
        let media_dir = if media_dir.is_empty() {
            self.media_root.join("_basic")
        } else {
            self.media_root.join(media_dir)
        };
        Ok(NotetypeView {
            id,
            name,
            kind,
            css,
            fields: serde_json::from_str(&fields).unwrap_or_default(),
            templates: serde_json::from_str(&templates).unwrap_or_default(),
            media_dir: media_dir.to_string_lossy().into_owned(),
        })
    }

    pub fn card_view(&self, card_id: i64) -> Result<CardView> {
        let (note_id, deck_id, ord, state, fields, tags, ntid, position): (
            i64,
            i64,
            i64,
            i64,
            String,
            String,
            i64,
            i64,
        ) = self
            .conn
            .query_row(
                "SELECT c.note_id, c.deck_id, c.ord, c.state, n.fields, n.tags, n.notetype_id, c.position
                 FROM cards c JOIN notes n ON n.id = c.note_id WHERE c.id = ?",
                [card_id],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                        r.get(6)?,
                        r.get(7)?,
                    ))
                },
            )
            .optional()?
            .ok_or_else(|| anyhow!("卡片不存在"))?;
        Ok(CardView {
            card_id,
            note_id,
            deck_id,
            deck_name: self.deck_name(deck_id)?,
            ord,
            state,
            position,
            fields: serde_json::from_str(&fields).unwrap_or_default(),
            tags,
            notetype: self.notetype(ntid)?,
        })
    }

    /// 手动添加一个单词（正面/背面）
    pub fn add_basic_note(&self, deck_id: i64, front: &str, back: &str, tags: &str) -> Result<i64> {
        if util::strip_html(front).is_empty() && !front.contains("<img") {
            return Err(anyhow!("正面不能为空"));
        }
        self.deck_config(deck_id)?;
        let now = util::now_secs();
        let tx = self.conn.unchecked_transaction()?;
        let guid = format!("ankiki-{}-{}", util::now_millis(), rand::random::<u32>());
        tx.execute(
            "INSERT INTO notes (guid, deck_id, notetype_id, fields, tags, sort_field, created_at, modified_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                guid,
                deck_id,
                BASIC_NOTETYPE_ID,
                serde_json::to_string(&[front, back])?,
                tags.trim(),
                util::strip_html(front),
                now,
                now
            ],
        )?;
        let note_id = tx.last_insert_rowid();
        let pos: i64 = tx.query_row(
            "SELECT MAX(COALESCE(MAX(position), -1), -1) + 1 FROM cards WHERE deck_id = ?",
            [deck_id],
            |r| r.get(0),
        )?;
        tx.execute(
            "INSERT INTO cards (note_id, deck_id, ord, state, due, position, created_at)
             VALUES (?, ?, 0, 0, 0, ?, ?)",
            params![note_id, deck_id, pos, now],
        )?;
        tx.commit()?;
        Ok(note_id)
    }

    pub fn update_note(&self, note_id: i64, fields: Vec<String>, tags: &str) -> Result<()> {
        let ntid: i64 = self
            .conn
            .query_row("SELECT notetype_id FROM notes WHERE id = ?", [note_id], |r| {
                r.get(0)
            })
            .optional()?
            .ok_or_else(|| anyhow!("笔记不存在"))?;
        let sort_idx: i64 = self.conn.query_row(
            "SELECT sort_idx FROM notetypes WHERE id = ?",
            [ntid],
            |r| r.get(0),
        )?;
        let sort_field = fields
            .get(sort_idx as usize)
            .or_else(|| fields.first())
            .map(|f| util::strip_html(f))
            .unwrap_or_default();
        self.conn.execute(
            "UPDATE notes SET fields = ?, tags = ?, sort_field = ?, modified_at = ? WHERE id = ?",
            params![
                serde_json::to_string(&fields)?,
                tags.trim(),
                sort_field,
                util::now_secs(),
                note_id
            ],
        )?;
        Ok(())
    }

    pub fn delete_notes(&mut self, note_ids: &[i64]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        for id in note_ids {
            tx.execute(
                "DELETE FROM revlog WHERE card_id IN (SELECT id FROM cards WHERE note_id = ?)",
                [id],
            )?;
            tx.execute("DELETE FROM cards WHERE note_id = ?", [id])?;
            tx.execute("DELETE FROM notes WHERE id = ?", [id])?;
        }
        tx.commit()?;
        self.undo = None;
        Ok(())
    }

    /// 标为熟词（或取消）：熟词不会出现在每日新词和复习中
    pub fn set_known(&mut self, card_ids: &[i64], known: bool) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        {
            let mut stmt = if known {
                tx.prepare("UPDATE cards SET suspended = 2 WHERE id = ?")?
            } else {
                tx.prepare("UPDATE cards SET suspended = 0 WHERE id = ? AND suspended = 2")?
            };
            for id in card_ids {
                stmt.execute([id])?;
            }
        }
        tx.commit()?;
        self.undo = None;
        Ok(())
    }

    /// 删除这些卡片所属的笔记
    pub fn delete_notes_of_cards(&mut self, card_ids: &[i64]) -> Result<()> {
        let mut note_ids = Vec::new();
        {
            let mut stmt = self.conn.prepare("SELECT note_id FROM cards WHERE id = ?")?;
            for id in card_ids {
                if let Some(n) = stmt.query_row([id], |r| r.get::<_, i64>(0)).optional()? {
                    note_ids.push(n);
                }
            }
        }
        note_ids.sort_unstable();
        note_ids.dedup();
        self.delete_notes(&note_ids)
    }

    pub fn set_suspended(&mut self, card_ids: &[i64], suspended: bool) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        for id in card_ids {
            tx.execute(
                "UPDATE cards SET suspended = ? WHERE id = ?",
                params![suspended as i64, id],
            )?;
        }
        tx.commit()?;
        self.undo = None;
        Ok(())
    }

    /// 重置为新卡（清空学习进度）
    pub fn forget_cards(&mut self, card_ids: &[i64]) -> Result<()> {
        let tx = self.conn.unchecked_transaction()?;
        for id in card_ids {
            let deck_id: Option<i64> = tx
                .query_row("SELECT deck_id FROM cards WHERE id = ?", [id], |r| r.get(0))
                .optional()?;
            let Some(deck_id) = deck_id else { continue };
            let pos: i64 = tx.query_row(
                "SELECT MAX(COALESCE(MAX(position), -1), -1) + 1 FROM cards WHERE deck_id = ?",
                [deck_id],
                |r| r.get(0),
            )?;
            tx.execute(
                "UPDATE cards SET state = 0, due = 0, interval = 0, step = 0, stability = 0,
                 difficulty = 0, reps = 0, lapses = 0, last_review = 0, position = ? WHERE id = ?",
                params![pos, id],
            )?;
        }
        tx.commit()?;
        self.undo = None;
        Ok(())
    }
}

pub(crate) fn parse_config(raw: &str) -> DeckConfig {
    serde_json::from_str(raw).unwrap_or_default()
}

const BASIC_CSS: &str = r#".card {
  font-family: -apple-system, "PingFang SC", "Helvetica Neue", sans-serif;
  font-size: 22px;
  text-align: center;
  color: #1f2330;
  background-color: #ffffff;
  line-height: 1.6;
}
.word { font-size: 40px; font-weight: 700; letter-spacing: 0.5px; }
.meaning { font-size: 22px; color: #3a4154; }
hr#answer { border: none; border-top: 1px solid #e3e6ee; margin: 24px auto; width: 60%; }
.nightMode.card, .night_mode .card { color: #e9ebf2; background-color: #1d2130; }
.nightMode .meaning { color: #c3c8d6; }
.nightMode hr#answer { border-top-color: #384057; }
"#;

#[cfg(test)]
pub(crate) fn test_collection() -> (Collection, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let col = Collection::open(dir.path()).unwrap();
    (col, dir)
}
