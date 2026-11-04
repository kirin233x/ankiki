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
        c.last_review, c.created_at, c.position
    FROM cards c
    JOIN notes n ON n.id = c.note_id
    JOIN decks d ON d.id = c.deck_id
    JOIN notetypes nt ON nt.id = n.notetype_id";

fn row_from(r: &rusqlite::Row) -> rusqlite::Result<(CardRow, i64, i64)> {
    let fields_json: String = r.get(5)?;
    let sort_idx: i64 = r.get(6)?;
    let fields: Vec<String> = serde_json::from_str(&fields_json).unwrap_or_default();
    // 「释义」列：优先选含中文的字段，否则取第一个非空字段
    let others: Vec<String> = fields
        .iter()
        .enumerate()
        .filter(|(i, _)| *i as i64 != sort_idx)
        .map(|(_, f)| strip_html(f))
        .filter(|s| !s.is_empty())
        .collect();
    let has_cjk = |s: &str| s.chars().any(|c| ('\u{4e00}'..='\u{9fff}').contains(&c));
    let back = others
        .iter()
        .find(|s| has_cjk(s))
        .or_else(|| others.first())
        .cloned()
        .unwrap_or_default();
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
            suspended: r.get::<_, i64>(14)? != 0,
            tags: r.get(15)?,
        },
        r.get(16)?,
        r.get(17)?,
    ))
}

fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

impl Collection {
    pub fn search_cards(&self, q: &SearchQuery) -> Result<SearchResult> {
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
            "due" => {
                where_sql.push("c.state != 0 AND c.suspended = 0 AND c.due < ?".into());
                args.push(day_end.into());
            }
            _ => {}
        }
        let where_clause = where_sql.join(" AND ");
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
    use crate::models::SearchQuery;
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
