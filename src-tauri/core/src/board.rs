//! 词表看板：按学习日（或全部已学）列出单词与中文释义，并按掌握情况标色。
//!
//! 某一天/某段时间：按这段时间内最差的一次评分标色 —— 重来/困难 红，良好 绿，简单 蓝。
//! 全部已学：按当前记忆状态标色 —— 难词/易忘 红，已掌握（间隔 ≥ 21 天）蓝，其他 绿，熟词 灰。

use crate::browse::extract_meaning;
use crate::models::*;
use crate::util::{self, date_start, study_date};
use crate::{parse_config, Collection};
use anyhow::{anyhow, Result};
use chrono::{Duration, NaiveDate};
use rusqlite::params;
use std::collections::HashMap;

fn parse_date(s: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").map_err(|_| anyhow!("日期格式应为 YYYY-MM-DD"))
}

fn color_rank(c: &str) -> u8 {
    match c {
        "red" => 0,
        "green" => 1,
        "blue" => 2,
        _ => 3,
    }
}

const COLS: &str = "c.note_id, c.deck_id, c.interval, c.suspended, c.difficulty, c.lapses, c.state,
    n.fields, n.sort_field, nt.sort_idx, nt.fields, d.config";

impl Collection {
    pub fn word_board(&self, q: &BoardQuery) -> Result<BoardResult> {
        let rollover = self.rollover();
        let today = study_date(util::now_secs(), rollover);
        let range = match &q.from {
            Some(from) if !from.trim().is_empty() => {
                let from = parse_date(from)?;
                let to = match &q.to {
                    Some(t) if !t.trim().is_empty() => parse_date(t)?,
                    _ => from,
                };
                let (a, b) = if from <= to { (from, to) } else { (to, from) };
                Some((
                    date_start(a, rollover) * 1000,
                    date_start(b + Duration::days(1), rollover) * 1000,
                ))
            }
            _ => None,
        };

        let mut configs: HashMap<i64, String> = HashMap::new();
        let mut items: Vec<BoardItem> = Vec::new();
        let mut push = |r: &rusqlite::Row, card_id: i64, worst: i64, reviews: i64, was_new: bool, first_ts: i64, last_ts: i64| -> rusqlite::Result<()> {
            let note_id: i64 = r.get(6)?;
            let deck_id: i64 = r.get(7)?;
            let interval: i64 = r.get(8)?;
            let suspended: i64 = r.get(9)?;
            let difficulty: f64 = r.get(10)?;
            let lapses: i64 = r.get(11)?;
            let state: i64 = r.get(12)?;
            let fields: Vec<String> = serde_json::from_str(&r.get::<_, String>(13)?).unwrap_or_default();
            let word: String = r.get(14)?;
            let sort_idx: i64 = r.get(15)?;
            let names: Vec<String> = serde_json::from_str(&r.get::<_, String>(16)?).unwrap_or_default();
            let preferred = configs
                .entry(deck_id)
                .or_insert_with(|| parse_config(&r.get::<_, String>(17).unwrap_or_default()).meaning_field)
                .clone();
            let meaning: String = extract_meaning(&fields, sort_idx, &names, &preferred)
                .chars()
                .take(200)
                .collect();
            let known = suspended == 2;
            let color = if worst > 0 {
                match worst {
                    1 | 2 => "red",
                    3 => "green",
                    _ => "blue",
                }
            } else if known {
                "gray"
            } else if state == STATE_LEARNING
                || state == STATE_RELEARNING
                || lapses >= 3
                || difficulty >= 7.0
            {
                "red"
            } else if interval >= 21 {
                "blue"
            } else {
                "green"
            };
            items.push(BoardItem {
                card_id,
                note_id,
                deck_id,
                word,
                meaning,
                color: color.into(),
                is_new: was_new,
                known,
                reviews,
                worst_rating: worst,
                interval,
                first_ts,
                last_ts,
            });
            Ok(())
        };

        match range {
            Some((start, end)) => {
                let sql = format!(
                    "SELECT g.card_id, g.worst, g.cnt, g.was_new, g.first_ts, g.last_ts, {COLS}
                     FROM (SELECT card_id, MIN(rating) AS worst, COUNT(*) AS cnt, MAX(state = 0) AS was_new,
                                  MIN(ts) AS first_ts, MAX(ts) AS last_ts
                           FROM revlog WHERE ts >= ?1 AND ts < ?2 AND (?3 IS NULL OR deck_id = ?3)
                           GROUP BY card_id) g
                     JOIN cards c ON c.id = g.card_id
                     JOIN notes n ON n.id = c.note_id
                     JOIN notetypes nt ON nt.id = n.notetype_id
                     JOIN decks d ON d.id = c.deck_id
                     ORDER BY g.first_ts"
                );
                let mut stmt = self.conn.prepare(&sql)?;
                let mut rows = stmt.query(params![start, end, q.deck_id])?;
                while let Some(r) = rows.next()? {
                    push(r, r.get(0)?, r.get(1)?, r.get(2)?, r.get::<_, i64>(3)? != 0, r.get(4)?, r.get(5)?)?;
                }
            }
            None => {
                let sql = format!(
                    "SELECT c.id, 0, c.reps, 0, c.last_review * 1000, c.last_review * 1000, {COLS}
                     FROM cards c
                     JOIN notes n ON n.id = c.note_id
                     JOIN notetypes nt ON nt.id = n.notetype_id
                     JOIN decks d ON d.id = c.deck_id
                     WHERE (c.state != 0 OR c.suspended = 2) AND (?1 IS NULL OR c.deck_id = ?1)
                     ORDER BY c.last_review DESC, c.id"
                );
                let mut stmt = self.conn.prepare(&sql)?;
                let mut rows = stmt.query(params![q.deck_id])?;
                while let Some(r) = rows.next()? {
                    push(r, r.get(0)?, 0, r.get(2)?, false, r.get(4)?, r.get(5)?)?;
                }
            }
        }

        // 同一个单词的多张卡片（如正反向）合并为一项，颜色取最差的
        let mut merged: Vec<BoardItem> = Vec::with_capacity(items.len());
        let mut index: HashMap<i64, usize> = HashMap::new();
        for it in items {
            match index.get(&it.note_id) {
                Some(&i) => {
                    let m = &mut merged[i];
                    if color_rank(&it.color) < color_rank(&m.color) {
                        m.color = it.color;
                        m.card_id = it.card_id;
                    }
                    m.is_new |= it.is_new;
                    m.known &= it.known;
                    m.reviews += it.reviews;
                    if it.worst_rating > 0 && (m.worst_rating == 0 || it.worst_rating < m.worst_rating) {
                        m.worst_rating = it.worst_rating;
                    }
                    m.first_ts = m.first_ts.min(it.first_ts);
                    m.last_ts = m.last_ts.max(it.last_ts);
                    m.interval = m.interval.max(it.interval);
                }
                None => {
                    index.insert(it.note_id, merged.len());
                    merged.push(it);
                }
            }
        }

        let mut res = BoardResult {
            today: today.format("%Y-%m-%d").to_string(),
            ..Default::default()
        };
        for it in &merged {
            match it.color.as_str() {
                "red" => res.red += 1,
                "green" => res.green += 1,
                "blue" => res.blue += 1,
                _ => res.gray += 1,
            }
            if it.is_new {
                res.new_count += 1;
            }
            if range.is_some() {
                res.reviews += it.reviews;
            }
        }
        res.items = merged;
        Ok(res)
    }

    /// 某本词书用到的全部字段名（用于选择「释义字段」）
    pub fn deck_fields(&self, deck_id: i64) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "SELECT fields FROM notetypes WHERE id IN (SELECT DISTINCT notetype_id FROM notes WHERE deck_id = ?)",
        )?;
        let mut out: Vec<String> = Vec::new();
        let rows = stmt.query_map([deck_id], |r| r.get::<_, String>(0))?;
        for raw in rows {
            let names: Vec<String> = serde_json::from_str(&raw?).unwrap_or_default();
            for n in names {
                if !out.contains(&n) {
                    out.push(n);
                }
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use crate::models::*;
    use crate::test_collection;

    #[test]
    fn board_today_and_all() {
        let (mut col, _d) = test_collection();
        let deck = col.create_deck("A").unwrap();
        for (w, m) in [("apple", "苹果"), ("banana", "<b>香蕉</b>"), ("cherry", "樱桃"), ("date", "枣")] {
            col.add_basic_note(deck, w, m, "").unwrap();
        }
        // apple: 重来后良好 → 红；banana: 良好 → 绿；cherry: 简单 → 蓝；date 未学
        let ratings = [vec![1, 3], vec![3], vec![4]];
        let st = col.study_state(deck).unwrap();
        let _ = st;
        let ids: Vec<i64> = col
            .search_cards(&SearchQuery { deck_id: Some(deck), sort: "sort".into(), ..Default::default() })
            .unwrap()
            .rows
            .iter()
            .map(|r| r.card_id)
            .collect();
        for (i, rs) in ratings.iter().enumerate() {
            for r in rs {
                col.answer_card(ids[i], *r, 1000).unwrap();
            }
        }
        let today = col.word_board(&BoardQuery { deck_id: None, from: None, to: None }).unwrap().today;
        let b = col
            .word_board(&BoardQuery { deck_id: Some(deck), from: Some(today.clone()), to: None })
            .unwrap();
        assert_eq!(b.items.len(), 3);
        let get = |w: &str| b.items.iter().find(|i| i.word == w).unwrap();
        assert_eq!(get("apple").color, "red");
        assert_eq!(get("apple").reviews, 2);
        assert_eq!(get("banana").color, "green");
        assert_eq!(get("banana").meaning, "香蕉");
        assert_eq!(get("cherry").color, "blue");
        assert!(get("cherry").is_new);
        assert_eq!((b.red, b.green, b.blue, b.new_count, b.reviews), (1, 1, 1, 3, 4));

        // 昨天没有记录
        let y = (chrono::NaiveDate::parse_from_str(&today, "%Y-%m-%d").unwrap() - chrono::Duration::days(1))
            .format("%Y-%m-%d")
            .to_string();
        assert!(col.word_board(&BoardQuery { deck_id: None, from: Some(y), to: None }).unwrap().items.is_empty());

        // 全部已学 + 熟词
        col.set_known(&[ids[3]], true).unwrap();
        let all = col.word_board(&BoardQuery::default()).unwrap();
        assert_eq!(all.items.len(), 4);
        assert_eq!(all.gray, 1);
        assert_eq!(all.items.iter().find(|i| i.word == "apple").unwrap().color, "red");

        // 指定释义字段
        let mut cfg = col.deck_config(deck).unwrap();
        cfg.meaning_field = "正面".into();
        col.update_deck_config(deck, cfg).unwrap();
        let b = col.word_board(&BoardQuery { deck_id: Some(deck), from: Some(today), to: None }).unwrap();
        assert_eq!(b.items[0].meaning, b.items[0].word);
        assert_eq!(col.deck_fields(deck).unwrap(), vec!["正面", "背面"]);
    }
}
