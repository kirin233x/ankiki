//! 学习队列与评分调度。
//!
//! 新卡 → 学习步长（默认 1 分钟、10 分钟）→ 毕业进入复习；
//! 复习间隔由 FSRS 记忆模型根据稳定性与期望保持率计算。

use crate::fsrs::{self, Memory};
use crate::models::*;
use crate::util::{self, day_start, study_date};
use crate::{Collection, UndoEntry};
use anyhow::{anyhow, Result};
use rand::Rng;
use rusqlite::{params, OptionalExtension};

/// 学习中的卡片在没有别的卡可学时，可以提前多少秒拿出来学
const LEARN_AHEAD_SECS: i64 = 20 * 60;

#[derive(Debug, Clone, PartialEq)]
pub struct Card {
    pub id: i64,
    pub note_id: i64,
    pub deck_id: i64,
    pub ord: i64,
    pub state: i64,
    pub due: i64,
    pub interval: i64,
    pub step: i64,
    pub stability: f64,
    pub difficulty: f64,
    pub reps: i64,
    pub lapses: i64,
    pub last_review: i64,
    pub suspended: bool,
    pub position: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub state: i64,
    pub step: i64,
    pub due: i64,
    pub interval: i64,
    pub memory: Memory,
    pub lapse: bool,
}

/// 计算给某张卡打出 `rating`（1 重来 / 2 困难 / 3 良好 / 4 简单）后的结果。纯函数，便于测试。
pub fn schedule(
    card: &Card,
    cfg: &DeckConfig,
    now: i64,
    rating: u8,
    rollover: u32,
    fuzz: bool,
) -> Outcome {
    let rating = rating.clamp(1, 4);
    let prev = (card.stability > 0.0).then(|| Memory {
        stability: card.stability,
        difficulty: if card.difficulty > 0.0 { card.difficulty } else { 5.0 },
    });
    let elapsed_days = if card.last_review > 0 {
        (study_date(now, rollover) - study_date(card.last_review, rollover))
            .num_days()
            .max(0) as f64
    } else {
        0.0
    };
    let mem = |r: u8| fsrs::next_memory(prev, elapsed_days, r);
    let max_ivl = cfg.max_interval.max(1);
    let ivl_for = |m: &Memory| -> i64 {
        (fsrs::interval_days(m.stability, cfg.desired_retention).round() as i64).clamp(1, max_ivl)
    };
    let day0 = day_start(now, rollover);
    let review = |ivl: i64, r: u8, lapse: bool| {
        let ivl = if fuzz { fuzz_interval(ivl, max_ivl) } else { ivl };
        Outcome {
            state: STATE_REVIEW,
            step: 0,
            due: day0 + ivl * 86400,
            interval: ivl,
            memory: mem(r),
            lapse,
        }
    };

    match card.state {
        STATE_REVIEW => {
            let mut hard = ivl_for(&mem(2));
            let mut good = ivl_for(&mem(3));
            let mut easy = ivl_for(&mem(4));
            hard = hard.min(good);
            good = good.max(hard + 1).min(max_ivl);
            easy = easy.max(good + 1).min(max_ivl);
            match rating {
                1 => {
                    if let Some(first) = cfg.relearning_steps.first() {
                        Outcome {
                            state: STATE_RELEARNING,
                            step: 0,
                            due: now + step_secs(*first),
                            interval: ivl_for(&mem(1)),
                            memory: mem(1),
                            lapse: true,
                        }
                    } else {
                        review(ivl_for(&mem(1)), 1, true)
                    }
                }
                2 => review(hard, 2, false),
                3 => review(good, 3, false),
                _ => review(easy, 4, false),
            }
        }
        _ => {
            let relearn = card.state == STATE_RELEARNING;
            let steps = if relearn {
                &cfg.relearning_steps
            } else {
                &cfg.learning_steps
            };
            let step = if card.state == STATE_NEW {
                0
            } else {
                card.step.max(0) as usize
            };
            let learn_state = if relearn {
                STATE_RELEARNING
            } else {
                STATE_LEARNING
            };
            let good_ivl = ivl_for(&mem(3));
            let easy_ivl = ivl_for(&mem(4)).max(good_ivl + 1).min(max_ivl);
            let learn = |step: usize, delay_mins: f64, r: u8| Outcome {
                state: learn_state,
                step: step as i64,
                due: now + step_secs(delay_mins),
                interval: if relearn { card.interval } else { 0 },
                memory: mem(r),
                lapse: false,
            };
            if steps.is_empty() {
                return match rating {
                    1 => review(1, 1, false),
                    2 => review(ivl_for(&mem(2)).min(good_ivl), 2, false),
                    3 => review(good_ivl, 3, false),
                    _ => review(easy_ivl, 4, false),
                };
            }
            let step = step.min(steps.len() - 1);
            match rating {
                1 => learn(0, steps[0], 1),
                2 => {
                    let delay = if step == 0 {
                        if steps.len() > 1 {
                            (steps[0] + steps[1]) / 2.0
                        } else {
                            steps[0] * 1.5
                        }
                    } else {
                        steps[step]
                    };
                    learn(step, delay, 2)
                }
                3 => {
                    if step + 1 < steps.len() {
                        learn(step + 1, steps[step + 1], 3)
                    } else {
                        review(good_ivl, 3, false)
                    }
                }
                _ => review(easy_ivl, 4, false),
            }
        }
    }
}

fn step_secs(mins: f64) -> i64 {
    (mins * 60.0).round().max(1.0) as i64
}

fn fuzz_interval(ivl: i64, max_ivl: i64) -> i64 {
    if ivl < 3 {
        return ivl;
    }
    let delta = if ivl < 7 {
        1
    } else {
        ((ivl as f64) * 0.06).round().max(1.0) as i64
    };
    let v = rand::thread_rng().gen_range(ivl - delta..=ivl + delta);
    v.clamp(1, max_ivl)
}

/// 把间隔格式化为按钮上显示的文字
pub fn format_secs(secs: i64) -> String {
    if secs < 60 {
        "<1分钟".into()
    } else if secs < 3600 {
        format!("{}分钟", (secs as f64 / 60.0).round() as i64)
    } else if secs < 86400 {
        format!("{}小时", trim_num(secs as f64 / 3600.0))
    } else {
        format_days(((secs as f64) / 86400.0).round() as i64)
    }
}

pub fn format_days(days: i64) -> String {
    if days < 30 {
        format!("{}天", days.max(1))
    } else if days < 365 {
        format!("{}个月", trim_num(days as f64 / 30.0))
    } else {
        format!("{}年", trim_num(days as f64 / 365.0))
    }
}

fn trim_num(v: f64) -> String {
    let s = format!("{:.1}", v);
    s.trim_end_matches(".0").to_string()
}

impl Collection {
    pub fn load_card(&self, id: i64) -> Result<Card> {
        self.conn
            .query_row(
                "SELECT id, note_id, deck_id, ord, state, due, interval, step, stability, difficulty,
                        reps, lapses, last_review, suspended, position
                 FROM cards WHERE id = ?",
                [id],
                |r| {
                    Ok(Card {
                        id: r.get(0)?,
                        note_id: r.get(1)?,
                        deck_id: r.get(2)?,
                        ord: r.get(3)?,
                        state: r.get(4)?,
                        due: r.get(5)?,
                        interval: r.get(6)?,
                        step: r.get(7)?,
                        stability: r.get(8)?,
                        difficulty: r.get(9)?,
                        reps: r.get(10)?,
                        lapses: r.get(11)?,
                        last_review: r.get(12)?,
                        suspended: r.get::<_, i64>(13)? != 0,
                        position: r.get(14)?,
                    })
                },
            )
            .optional()?
            .ok_or_else(|| anyhow!("卡片不存在"))
    }

    fn save_card(&self, c: &Card) -> Result<()> {
        self.conn.execute(
            "UPDATE cards SET state = ?, due = ?, interval = ?, step = ?, stability = ?,
                 difficulty = ?, reps = ?, lapses = ?, last_review = ?, suspended = ?, position = ?
             WHERE id = ?",
            params![
                c.state,
                c.due,
                c.interval,
                c.step,
                c.stability,
                c.difficulty,
                c.reps,
                c.lapses,
                c.last_review,
                c.suspended as i64,
                c.position,
                c.id
            ],
        )?;
        Ok(())
    }

    fn today_start_ms(&self) -> i64 {
        day_start(util::now_secs(), self.rollover()) * 1000
    }

    /// 今天已作答次数
    pub(crate) fn studied_today(&self, deck_id: Option<i64>) -> Result<i64> {
        let start = self.today_start_ms();
        Ok(match deck_id {
            Some(d) => self.conn.query_row(
                "SELECT COUNT(*) FROM revlog WHERE deck_id = ? AND ts >= ?",
                params![d, start],
                |r| r.get(0),
            )?,
            None => self.conn.query_row(
                "SELECT COUNT(*) FROM revlog WHERE ts >= ?",
                [start],
                |r| r.get(0),
            )?,
        })
    }

    fn limits_left(&self, deck_id: i64, cfg: &DeckConfig) -> Result<(i64, i64)> {
        let start = self.today_start_ms();
        let (new_done, rev_done): (i64, i64) = self.conn.query_row(
            "SELECT COUNT(DISTINCT CASE WHEN state = 0 THEN card_id END),
                    COALESCE(SUM(state = 2), 0)
             FROM revlog WHERE deck_id = ? AND ts >= ?",
            params![deck_id, start],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        Ok((
            (cfg.new_per_day - new_done).max(0),
            (cfg.reviews_per_day - rev_done).max(0),
        ))
    }

    pub(crate) fn deck_counts(&self, deck_id: i64, cfg: &DeckConfig) -> Result<Counts> {
        let now = util::now_secs();
        let day_end = day_start(now, self.rollover()) + 86400;
        let (new_left, rev_left) = self.limits_left(deck_id, cfg)?;
        let (new_avail, learning, review): (i64, i64, i64) = self.conn.query_row(
            "SELECT COALESCE(SUM(state = 0), 0),
                    COALESCE(SUM(state IN (1, 3) AND due < ?2), 0),
                    COALESCE(SUM(state = 2 AND due < ?2), 0)
             FROM cards WHERE deck_id = ?1 AND suspended = 0",
            params![deck_id, day_end],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        Ok(Counts {
            new: new_avail.min(new_left),
            learning,
            review: review.min(rev_left),
        })
    }

    fn pick_one(&self, sql: &str, p: impl rusqlite::Params) -> Result<Option<i64>> {
        Ok(self.conn.query_row(sql, p, |r| r.get(0)).optional()?)
    }

    /// 选出下一张要学的卡
    fn next_card_id(&self, deck_id: i64, counts: &Counts) -> Result<(Option<i64>, Option<i64>)> {
        let now = util::now_secs();
        let day_end = day_start(now, self.rollover()) + 86400;
        // 1. 已到期的学习中卡片
        if let Some(id) = self.pick_one(
            "SELECT id FROM cards WHERE deck_id = ? AND state IN (1, 3) AND suspended = 0
             AND due <= ? ORDER BY due LIMIT 1",
            params![deck_id, now],
        )? {
            return Ok((Some(id), None));
        }
        // 2. 复习卡与新卡按剩余数量比例穿插
        let review = if counts.review > 0 {
            self.pick_one(
                "SELECT id FROM cards WHERE deck_id = ? AND state = 2 AND suspended = 0
                 AND due < ? ORDER BY due, id LIMIT 1",
                params![deck_id, day_end],
            )?
        } else {
            None
        };
        let new = if counts.new > 0 {
            self.pick_one(
                "SELECT id FROM cards WHERE deck_id = ? AND state = 0 AND suspended = 0
                 ORDER BY position, id LIMIT 1",
                [deck_id],
            )?
        } else {
            None
        };
        let chosen = match (review, new) {
            (Some(r), Some(n)) => {
                let p_new = counts.new as f64 / (counts.new + counts.review) as f64;
                if rand::thread_rng().gen_bool(p_new.clamp(0.0, 1.0)) {
                    Some(n)
                } else {
                    Some(r)
                }
            }
            (r, n) => r.or(n),
        };
        if chosen.is_some() {
            return Ok((chosen, None));
        }
        // 3. 没有其它卡时，提前学习 20 分钟内到期的学习卡
        if let Some(id) = self.pick_one(
            "SELECT id FROM cards WHERE deck_id = ? AND state IN (1, 3) AND suspended = 0
             AND due <= ? ORDER BY due LIMIT 1",
            params![deck_id, now + LEARN_AHEAD_SECS],
        )? {
            return Ok((Some(id), None));
        }
        // 4. 今天稍后到期的学习卡
        let next_due: Option<i64> = self.conn.query_row(
            "SELECT MIN(due) FROM cards WHERE deck_id = ? AND state IN (1, 3) AND suspended = 0
             AND due < ?",
            params![deck_id, day_end],
            |r| r.get(0),
        )?;
        Ok((None, next_due))
    }

    fn previews(&self, card_id: i64) -> Result<Vec<String>> {
        let card = self.load_card(card_id)?;
        let cfg = self.deck_config(card.deck_id)?;
        let now = util::now_secs();
        Ok((1..=4u8)
            .map(|r| {
                let o = schedule(&card, &cfg, now, r, self.rollover(), false);
                if o.state == STATE_REVIEW {
                    format_days(o.interval)
                } else {
                    format_secs(o.due - now)
                }
            })
            .collect())
    }

    fn build_state(&self, deck_id: i64, forced: Option<i64>) -> Result<StudyState> {
        let cfg = self.deck_config(deck_id)?;
        let counts = self.deck_counts(deck_id, &cfg)?;
        let (card_id, next_due_at) = match forced {
            Some(id) => (Some(id), None),
            None => self.next_card_id(deck_id, &counts)?,
        };
        let (card, previews) = match card_id {
            Some(id) => (Some(self.card_view(id)?), self.previews(id)?),
            None => (None, vec![]),
        };
        Ok(StudyState {
            card,
            counts,
            previews,
            next_due_at,
            studied_today: self.studied_today(Some(deck_id))?,
            can_undo: self
                .undo
                .as_ref()
                .map(|u| u.card.deck_id == deck_id)
                .unwrap_or(false),
        })
    }

    /// 获取某个词书当前的学习状态（下一张卡、剩余数量）
    pub fn study_state(&self, deck_id: i64) -> Result<StudyState> {
        self.build_state(deck_id, None)
    }

    /// 给卡片评分（1 重来 / 2 困难 / 3 良好 / 4 简单），返回下一张卡
    pub fn answer_card(&mut self, card_id: i64, rating: u8, duration_ms: i64) -> Result<StudyState> {
        if !(1..=4).contains(&rating) {
            return Err(anyhow!("评分必须在 1~4 之间"));
        }
        let before = self.load_card(card_id)?;
        let cfg = self.deck_config(before.deck_id)?;
        let now = util::now_secs();
        let o = schedule(&before, &cfg, now, rating, self.rollover(), true);
        let mut card = before.clone();
        card.state = o.state;
        card.step = o.step;
        card.due = o.due;
        card.interval = o.interval;
        card.stability = o.memory.stability;
        card.difficulty = o.memory.difficulty;
        card.reps += 1;
        if o.lapse {
            card.lapses += 1;
        }
        card.last_review = now;

        let tx = self.conn.unchecked_transaction()?;
        self.save_card(&card)?;
        let logged_ivl = if o.state == STATE_REVIEW {
            o.interval
        } else {
            -(o.due - now)
        };
        self.conn.execute(
            "INSERT INTO revlog (card_id, deck_id, ts, rating, state, interval, duration_ms)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                card.id,
                card.deck_id,
                util::now_millis(),
                rating,
                before.state,
                logged_ivl,
                duration_ms.clamp(0, 3_600_000)
            ],
        )?;
        let revlog_id = self.conn.last_insert_rowid();
        tx.commit()?;
        let deck_id = before.deck_id;
        self.undo = Some(UndoEntry {
            card: before,
            revlog_id,
        });
        self.build_state(deck_id, None)
    }

    /// 撤销上一次评分，返回被撤销的那张卡
    pub fn undo(&mut self) -> Result<Option<StudyState>> {
        let Some(entry) = self.undo.take() else {
            return Ok(None);
        };
        let tx = self.conn.unchecked_transaction()?;
        self.save_card(&entry.card)?;
        self.conn
            .execute("DELETE FROM revlog WHERE id = ?", [entry.revlog_id])?;
        tx.commit()?;
        Ok(Some(self.build_state(entry.card.deck_id, Some(entry.card.id))?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_collection;

    fn new_card() -> Card {
        Card {
            id: 1,
            note_id: 1,
            deck_id: 1,
            ord: 0,
            state: STATE_NEW,
            due: 0,
            interval: 0,
            step: 0,
            stability: 0.0,
            difficulty: 0.0,
            reps: 0,
            lapses: 0,
            last_review: 0,
            suspended: false,
            position: 0,
        }
    }

    #[test]
    fn learning_steps_then_graduate() {
        let cfg = DeckConfig::default();
        let now = 1_700_000_000;
        let c = new_card();
        let again = schedule(&c, &cfg, now, 1, 4, false);
        assert_eq!(again.state, STATE_LEARNING);
        assert_eq!(again.due - now, 60);
        let hard = schedule(&c, &cfg, now, 2, 4, false);
        assert_eq!(hard.due - now, 330);
        let good = schedule(&c, &cfg, now, 3, 4, false);
        assert_eq!((good.state, good.step, good.due - now), (STATE_LEARNING, 1, 600));
        let easy = schedule(&c, &cfg, now, 4, 4, false);
        assert_eq!(easy.state, STATE_REVIEW);
        assert!(easy.interval >= 2);

        // 10 分钟后再按良好 → 毕业
        let mut c2 = c.clone();
        c2.state = good.state;
        c2.step = good.step;
        c2.stability = good.memory.stability;
        c2.difficulty = good.memory.difficulty;
        c2.last_review = now;
        let grad = schedule(&c2, &cfg, now + 600, 3, 4, false);
        assert_eq!(grad.state, STATE_REVIEW);
        assert!(grad.interval >= 1);
    }

    #[test]
    fn review_order_and_lapse() {
        let cfg = DeckConfig::default();
        let now = 1_700_000_000;
        let mut c = new_card();
        c.state = STATE_REVIEW;
        c.stability = 10.0;
        c.difficulty = 5.0;
        c.interval = 10;
        c.last_review = now - 10 * 86400;
        let o: Vec<_> = (1..=4).map(|r| schedule(&c, &cfg, now, r, 4, false)).collect();
        assert_eq!(o[0].state, STATE_RELEARNING);
        assert!(o[0].lapse);
        assert!(o[1].interval < o[2].interval && o[2].interval < o[3].interval);
        assert!(o[2].interval > 10);
    }

    #[test]
    fn format() {
        assert_eq!(format_secs(30), "<1分钟");
        assert_eq!(format_secs(600), "10分钟");
        assert_eq!(format_secs(5400), "1.5小时");
        assert_eq!(format_days(3), "3天");
        assert_eq!(format_days(45), "1.5个月");
        assert_eq!(format_days(730), "2年");
    }

    #[test]
    fn study_flow_with_limits_and_undo() {
        let (mut col, _dir) = test_collection();
        let deck = col.create_deck("测试").unwrap();
        for i in 0..5 {
            col.add_basic_note(deck, &format!("word{i}"), "释义", "").unwrap();
        }
        let mut cfg = col.deck_config(deck).unwrap();
        cfg.new_per_day = 3;
        col.update_deck_config(deck, cfg).unwrap();

        let st = col.study_state(deck).unwrap();
        assert_eq!(st.counts.new, 3);
        assert_eq!(st.previews.len(), 4);
        let first = st.card.unwrap().card_id;

        // 简单 → 直接毕业
        let st = col.answer_card(first, 4, 1000).unwrap();
        assert_eq!(st.counts.new, 2);
        assert!(st.can_undo);
        let st2 = col.undo().unwrap().unwrap();
        assert_eq!(st2.card.unwrap().card_id, first);
        assert_eq!(st2.counts.new, 3);

        // 学完 3 张新卡（全部按简单）后今日完成
        let mut st = col.study_state(deck).unwrap();
        let mut n = 0;
        while let Some(card) = st.card.clone() {
            st = col.answer_card(card.card_id, 4, 500).unwrap();
            n += 1;
            assert!(n < 10);
        }
        assert_eq!(n, 3);
        assert_eq!(st.counts.new, 0);
        assert_eq!(st.studied_today, 3);
        let decks = col.list_decks().unwrap();
        assert_eq!(decks[0].learned_cards, 3);
    }
}
