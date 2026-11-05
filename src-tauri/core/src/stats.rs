//! 学习统计

use crate::models::*;
use crate::util::{self, day_start, study_date};
use crate::Collection;
use anyhow::Result;
use chrono::Duration;
use rusqlite::params;
use std::collections::{BTreeMap, HashMap};

const HEATMAP_DAYS: i64 = 53 * 7;

impl Collection {
    pub fn stats(&self, deck_id: Option<i64>) -> Result<Stats> {
        let now = util::now_secs();
        let rollover = self.rollover();
        let today0 = day_start(now, rollover);
        let today = study_date(now, rollover);
        let deck_filter = if deck_id.is_some() {
            " AND deck_id = ?2"
        } else {
            " AND (?2 IS NULL OR 1)"
        };
        let deck_arg = deck_id;
        let mut s = Stats::default();

        // 今日
        let (reviews, time_ms, again): (i64, i64, i64) = self.conn.query_row(
            &format!(
                "SELECT COUNT(*), COALESCE(SUM(duration_ms), 0), COALESCE(SUM(rating = 1), 0)
                 FROM revlog WHERE ts >= ?1{deck_filter}"
            ),
            params![today0 * 1000, deck_arg],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        s.today_reviews = reviews;
        s.today_time_ms = time_ms;
        s.today_again = again;
        s.today_new = self.conn.query_row(
            &format!(
                "SELECT COUNT(DISTINCT card_id) FROM revlog WHERE ts >= ?1 AND state = 0{deck_filter}"
            ),
            params![today0 * 1000, deck_arg],
            |r| r.get(0),
        )?;

        // 每日作答次数
        let mut per_day: BTreeMap<chrono::NaiveDate, i64> = BTreeMap::new();
        {
            let mut stmt = self.conn.prepare(&format!(
                "SELECT ts FROM revlog WHERE ?1 = ?1{deck_filter}"
            ))?;
            let mut rows = stmt.query(params![0, deck_arg])?;
            while let Some(r) = rows.next()? {
                let ts: i64 = r.get(0)?;
                *per_day.entry(study_date(ts / 1000, rollover)).or_default() += 1;
            }
        }
        s.total_reviews = per_day.values().sum();
        s.days_studied = per_day.len() as i64;
        s.avg_daily = if s.days_studied > 0 {
            s.total_reviews as f64 / s.days_studied as f64
        } else {
            0.0
        };
        // 连续学习天数：今天没学则从昨天往前算
        let mut day = if per_day.contains_key(&today) {
            today
        } else {
            today - Duration::days(1)
        };
        while per_day.contains_key(&day) {
            s.streak += 1;
            day -= Duration::days(1);
        }
        let start = today - Duration::days(HEATMAP_DAYS - 1);
        s.heatmap = (0..HEATMAP_DAYS)
            .map(|i| {
                let d = start + Duration::days(i);
                DayCount {
                    date: d.format("%Y-%m-%d").to_string(),
                    count: per_day.get(&d).copied().unwrap_or(0),
                }
            })
            .collect();

        // 卡片分布
        let card_filter = if deck_id.is_some() {
            "WHERE deck_id = ?1"
        } else {
            "WHERE (?1 IS NULL OR 1)"
        };
        let counts: (i64, i64, i64, i64, i64, i64, i64) = self.conn.query_row(
            &format!(
                "SELECT COUNT(*),
                    COALESCE(SUM(state = 0 AND suspended = 0), 0),
                    COALESCE(SUM(state IN (1, 3) AND suspended = 0), 0),
                    COALESCE(SUM(state = 2 AND interval < 21 AND suspended = 0), 0),
                    COALESCE(SUM(state = 2 AND interval >= 21 AND suspended = 0), 0),
                    COALESCE(SUM(suspended = 1), 0),
                    COALESCE(SUM(suspended = 2), 0)
                 FROM cards {card_filter}"
            ),
            params![deck_arg],
            |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?, r.get(6)?))
            },
        )?;
        (
            s.total_cards,
            s.new_cards,
            s.learning_cards,
            s.young_cards,
            s.mature_cards,
            s.suspended_cards,
            s.known_cards,
        ) = counts;

        // 未来 30 天到期预测 & 间隔分布
        s.forecast = vec![0; 30];
        s.intervals = vec![0; 8];
        {
            let mut stmt = self.conn.prepare(&format!(
                "SELECT state, due, interval FROM cards {card_filter} AND state != 0 AND suspended = 0"
            ))?;
            let mut rows = stmt.query(params![deck_arg])?;
            while let Some(r) = rows.next()? {
                let state: i64 = r.get(0)?;
                let due: i64 = r.get(1)?;
                let ivl: i64 = r.get(2)?;
                let idx = ((due - today0) as f64 / 86400.0).floor().max(0.0) as usize;
                if idx < 30 {
                    s.forecast[idx] += 1;
                }
                if state == STATE_REVIEW {
                    let bucket = match ivl {
                        i64::MIN..=0 => 0,
                        1..=2 => 1,
                        3..=6 => 2,
                        7..=13 => 3,
                        14..=29 => 4,
                        30..=89 => 5,
                        90..=179 => 6,
                        _ => 7,
                    };
                    s.intervals[bucket] += 1;
                }
            }
        }

        // 近 30 天保持率与评分分布
        let since = (today0 - 29 * 86400) * 1000;
        let mut ratings: HashMap<i64, i64> = HashMap::new();
        let (mut rev_total, mut rev_pass) = (0i64, 0i64);
        {
            let mut stmt = self.conn.prepare(&format!(
                "SELECT rating, state FROM revlog WHERE ts >= ?1{deck_filter}"
            ))?;
            let mut rows = stmt.query(params![since, deck_arg])?;
            while let Some(r) = rows.next()? {
                let rating: i64 = r.get(0)?;
                let state: i64 = r.get(1)?;
                *ratings.entry(rating).or_default() += 1;
                if state == STATE_REVIEW {
                    rev_total += 1;
                    if rating > 1 {
                        rev_pass += 1;
                    }
                }
            }
        }
        s.ratings_30d = (1..=4).map(|r| ratings.get(&r).copied().unwrap_or(0)).collect();
        s.retention_30d = (rev_total > 0).then(|| rev_pass as f64 / rev_total as f64);
        Ok(s)
    }
}

#[cfg(test)]
mod tests {
    use crate::test_collection;

    #[test]
    fn stats_after_study() {
        let (mut col, _d) = test_collection();
        let deck = col.create_deck("A").unwrap();
        for i in 0..3 {
            col.add_basic_note(deck, &format!("w{i}"), "x", "").unwrap();
        }
        let st = col.study_state(deck).unwrap();
        col.answer_card(st.card.unwrap().card_id, 4, 2000).unwrap();
        let s = col.stats(Some(deck)).unwrap();
        assert_eq!(s.today_reviews, 1);
        assert_eq!(s.today_new, 1);
        assert_eq!(s.streak, 1);
        assert_eq!(s.total_cards, 3);
        assert_eq!(s.new_cards, 2);
        assert_eq!(s.heatmap.len(), 371);
        assert_eq!(s.heatmap.last().unwrap().count, 1);
        let all = col.stats(None).unwrap();
        assert_eq!(all.today_reviews, 1);
    }
}
