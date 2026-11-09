//! 浏览 / 搜索全部单词

use crate::fsrs;
use crate::models::*;
use crate::util::{self, day_start, strip_html};
use crate::Collection;
use anyhow::{anyhow, Result};
use rusqlite::types::Value as SqlValue;
use rusqlite::OptionalExtension;

const ROW_SELECT: &str = "SELECT c.id, c.note_id, c.deck_id, d.name, n.sort_field, n.fields, nt.sort_idx,
        c.state, c.due, c.interval, c.stability, c.difficulty, c.reps, c.lapses, c.suspended, n.tags,
        c.last_review, c.created_at, c.position, nt.fields, d.config
    FROM cards c
    JOIN notes n ON n.id = c.note_id
    JOIN decks d ON d.id = c.deck_id
    JOIN notetypes nt ON nt.id = n.notetype_id";

fn row_from(r: &rusqlite::Row) -> rusqlite::Result<(CardRow, i64, i64)> {
    let fields_json: String = r.get(5)?;
    let sort_idx: i64 = r.get(6)?;
    let fields: Vec<String> = serde_json::from_str(&fields_json).unwrap_or_default();
    let names: Vec<String> = serde_json::from_str(&r.get::<_, String>(19)?).unwrap_or_default();
    let cfg = crate::parse_config(&r.get::<_, String>(20)?);
    let (word_idx, _) = extract_word(&fields, sort_idx, &names, &cfg.word_field);
    let back = extract_meaning(&fields, word_idx as i64, &names, &cfg.meaning_field);
    let back: String = back.chars().take(160).collect();
    let state: i64 = r.get(7)?;
    let due: i64 = r.get(8)?;
    let position: i64 = r.get(18)?;
    Ok((
        CardRow {
            card_id: r.get(0)?,
            note_id: r.get(1)?,
            deck_id: r.get(2)?,
            deck_name: r.get(3)?,
            front: r.get(4)?,
            back,
            state,
            due: if state == STATE_NEW { position } else { due },
            interval: r.get(9)?,
            stability: r.get(10)?,
            difficulty: r.get(11)?,
            reps: r.get(12)?,
            lapses: r.get(13)?,
            suspended: r.get::<_, i64>(14)? == 1,
            known: r.get::<_, i64>(14)? == 2,
            tags: r.get(15)?,
        },
        r.get(16)?,
        r.get(17)?,
    ))
}

/// 常见的「单词」字段名
const WORD_FIELD_NAMES: &[&str] = &[
    "word", "words", "单词", "词汇", "单字", "英文", "英语", "english", "front", "正面", "expression",
    "vocab", "vocabulary", "term", "headword", "lemma", "词条", "生词",
];

/// 看起来像一个单词：非空、含字母或汉字等文字、不是音标、不太长
fn word_like(s: &str) -> bool {
    let t = s.trim();
    !t.is_empty()
        && t.chars().any(|c| c.is_alphabetic())
        && !t.starts_with('/')
        && !t.starts_with('[')
        && t.chars().count() <= 60
}

/// 取一条笔记的「单词」：返回 (字段序号, 纯文本)。
/// 优先使用指定字段；再按字段名识别（Word/单词/Front…）；再用排序字段（如果不是序号）；
/// 再取第一个像单词的字段。
pub(crate) fn extract_word(
    fields: &[String],
    sort_idx: i64,
    field_names: &[String],
    preferred: &str,
) -> (usize, String) {
    let text = |i: usize| fields.get(i).map(|f| strip_html(f)).unwrap_or_default();
    if !preferred.is_empty() {
        if let Some(i) = field_names.iter().position(|n| n == preferred) {
            let v = text(i);
            if !v.is_empty() {
                return (i, v);
            }
        }
    }
    for (i, name) in field_names.iter().enumerate() {
        let n = name.trim().to_lowercase();
        if WORD_FIELD_NAMES.contains(&n.as_str()) {
            let v = text(i);
            if word_like(&v) {
                return (i, v);
            }
        }
    }
    let si = sort_idx.max(0) as usize;
    let sv = text(si);
    if word_like(&sv) {
        return (si, sv);
    }
    for i in 0..fields.len() {
        let v = text(i);
        if word_like(&v) {
            return (i, v);
        }
    }
    (si, sv)
}

/// 取一条笔记的「释义」：优先使用指定字段；否则选第一个含中文的字段，再否则取第一个非空字段
pub(crate) fn extract_meaning(
    fields: &[String],
    sort_idx: i64,
    field_names: &[String],
    preferred: &str,
) -> String {
    if !preferred.is_empty() {
        if let Some(i) = field_names.iter().position(|n| n == preferred) {
            let v = fields.get(i).map(|f| strip_html(f)).unwrap_or_default();
            if !v.is_empty() {
                return v;
            }
        }
    }
    let others: Vec<String> = fields
        .iter()
        .enumerate()
        .filter(|(i, _)| *i as i64 != sort_idx)
        .map(|(_, f)| strip_html(f))
        .filter(|s| !s.is_empty())
        .collect();
    let has_cjk = |s: &str| s.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c));
    others
        .iter()
        .find(|s| has_cjk(s))
        .or_else(|| others.first())
        .cloned()
        .unwrap_or_default()
}

fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

impl Collection {
    fn search_where(&self, q: &SearchQuery) -> (String, Vec<SqlValue>) {
        let mut where_sql = vec!["1 = 1".to_string()];
        let mut args: Vec<SqlValue> = vec![];
        if let Some(d) = q.deck_id {
            where_sql.push("c.deck_id = ?".into());
            args.push(d.into());
        }
        let text = q.text.trim();
        if !text.is_empty() {
            for word in text.split_whitespace() {
                let pat = format!("%{}%", escape_like(word));
                where_sql.push(
                    "(n.sort_field LIKE ? ESCAPE '\\' OR n.fields LIKE ? ESCAPE '\\' OR n.tags LIKE ? ESCAPE '\\')"
                        .into(),
                );
                args.push(pat.clone().into());
                args.push(pat.clone().into());
                args.push(pat.into());
            }
        }
        let day_end = day_start(util::now_secs(), self.rollover()) + 86400;
        match q.state.as_str() {
            "new" => where_sql.push("c.state = 0 AND c.suspended = 0".into()),
            "learning" => where_sql.push("c.state IN (1, 3) AND c.suspended = 0".into()),
            "review" => where_sql.push("c.state = 2 AND c.suspended = 0".into()),
            "suspended" => where_sql.push("c.suspended = 1".into()),
            "known" => where_sql.push("c.suspended = 2".into()),
            "due" => {
                where_sql.push("c.state != 0 AND c.suspended = 0 AND c.due < ?".into());
                args.push(day_end.into());
            }
            _ => {}
        }
        (where_sql.join(" AND "), args)
    }

    /// 符合搜索条件的全部卡片 id（用于「全选」批量操作）
    pub fn search_card_ids(&self, q: &SearchQuery) -> Result<Vec<i64>> {
        let (where_clause, args) = self.search_where(q);
        let mut stmt = self.conn.prepare(&format!(
            "SELECT c.id FROM cards c JOIN notes n ON n.id = c.note_id WHERE {where_clause} ORDER BY c.id"
        ))?;
        let ids = stmt
            .query_map(rusqlite::params_from_iter(args.iter()), |r| r.get(0))?
            .collect::<std::result::Result<Vec<i64>, _>>()?;
        Ok(ids)
    }

    pub fn search_cards(&self, q: &SearchQuery) -> Result<SearchResult> {
        let (where_clause, args) = self.search_where(q);
        let order = match q.sort.as_str() {
            "sort" => "n.sort_field COLLATE NOCASE",
            "due" => "(c.state = 0), CASE WHEN c.state = 0 THEN c.position ELSE c.due END",
            "interval" => "c.interval",
            "difficulty" => "c.difficulty",
            "reps" => "c.reps",
            "lapses" => "c.lapses",
            _ => "c.id",
        };
        let dir = if q.desc { "DESC" } else { "ASC" };
        let limit = if q.limit <= 0 { 50 } else { q.limit.min(500) };

        let total: i64 = self.conn.query_row(
            &format!(
                "SELECT COUNT(*) FROM cards c JOIN notes n ON n.id = c.note_id WHERE {where_clause}"
            ),
            rusqlite::params_from_iter(args.iter()),
            |r| r.get(0),
        )?;
        let sql = format!(
            "{ROW_SELECT} WHERE {where_clause} ORDER BY {order} {dir}, c.id {dir} LIMIT {limit} OFFSET {}",
            q.offset.max(0)
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt
            .query_map(rusqlite::params_from_iter(args.iter()), row_from)?
            .map(|r| r.map(|x| x.0))
            .collect::<std::result::Result<Vec<_>, _>>()?;
        Ok(SearchResult { total, rows })
    }

    pub fn card_detail(&self, card_id: i64) -> Result<CardDetail> {
        let (row, last_review, created_at) = self
            .conn
            .query_row(&format!("{ROW_SELECT} WHERE c.id = ?"), [card_id], row_from)
            .optional()?
            .ok_or_else(|| anyhow!("卡片不存在"))?;
        let retrievability = (row.stability > 0.0 && last_review > 0 && row.state != STATE_NEW)
            .then(|| {
                let elapsed = (util::now_secs() - last_review) as f64 / 86400.0;
                fsrs::retrievability(elapsed, row.stability)
            });
        let history = {
            let mut stmt = self.conn.prepare(
                "SELECT ts, rating, state, interval, duration_ms FROM revlog
                 WHERE card_id = ? ORDER BY ts DESC LIMIT 100",
            )?;
            let rows = stmt.query_map([card_id], |r| {
                Ok(RevlogEntry {
                    ts: r.get(0)?,
                    rating: r.get(1)?,
                    state: r.get(2)?,
                    interval: r.get(3)?,
                    duration_ms: r.get(4)?,
                })
            })?;
            rows.collect::<std::result::Result<Vec<_>, _>>()?
        };
        Ok(CardDetail {
            view: self.card_view(card_id)?,
            row,
            retrievability,
            last_review,
            created_at,
            history,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{extract_meaning, extract_word};
    use crate::models::SearchQuery;

    fn v(xs: &[&str]) -> Vec<String> {
        xs.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn word_detection() {
        // 排序字段是序号：跳过
        let f = v(&["12", "abandon", "/əˈbændən/", "v. 放弃"]);
        let n = v(&["序号", "Spelling", "音标", "释义"]);
        assert_eq!(extract_word(&f, 0, &n, ""), (1, "abandon".into()));
        assert_eq!(extract_meaning(&f, 1, &n, ""), "v. 放弃");
        // 字段名识别
        let n2 = v(&["编号", "音标", "Word", "释义"]);
        let f2 = v(&["7", "/x/", "<b>apple</b>", "苹果"]);
        assert_eq!(extract_word(&f2, 0, &n2, "").1, "apple");
        // 手动指定
        assert_eq!(extract_word(&f, 0, &n, "释义").1, "v. 放弃");
        // 普通情况保持排序字段
        assert_eq!(extract_word(&v(&["dog", "狗"]), 0, &v(&["Front", "Back"]), "").0, 0);
    }
    use crate::test_collection;

    #[test]
    fn search() {
        let (mut col, _d) = test_collection();
        let deck = col.create_deck("A").unwrap();
        col.add_basic_note(deck, "apple", "<b>苹果</b>", "fruit").unwrap();
        col.add_basic_note(deck, "banana", "香蕉", "").unwrap();
        col.add_basic_note(deck, "100%_sure", "确定", "").unwrap();
        let r = col
            .search_cards(&SearchQuery {
                text: "苹果".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(r.total, 1);
        assert_eq!(r.rows[0].front, "apple");
        assert_eq!(r.rows[0].back, "苹果");
        let r = col
            .search_cards(&SearchQuery {
                text: "%_".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(r.total, 1);
        let r = col
            .search_cards(&SearchQuery {
                deck_id: Some(deck),
                sort: "sort".into(),
                desc: true,
                ..Default::default()
            })
            .unwrap();
        assert_eq!(r.total, 3);
        assert_eq!(r.rows[0].front, "banana");
        let id = r.rows[0].card_id;
        col.set_suspended(&[id], true).unwrap();
        let r = col
            .search_cards(&SearchQuery {
                state: "suspended".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(r.total, 1);
        let detail = col.card_detail(id).unwrap();
        assert_eq!(detail.view.fields[0], "banana");
    }
}
