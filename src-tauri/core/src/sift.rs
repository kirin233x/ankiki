//! 快速过词：把词书里的新词快速过一遍，认识的标为熟词（不再学习），
//! 不认识的排到学习队列最前面。
//!
//! 「已过」的新卡用负数 position 标记（越早过的越靠前），未过的 position ≥ 0。

use crate::models::*;
use crate::Collection;
use anyhow::Result;
use rusqlite::params;

const KEEP_BASE: i64 = -1_000_000_000;

impl Collection {
    /// 取一批还没过的新词
    pub fn sift_batch(&self, deck_id: i64, limit: i64, exclude: &[i64]) -> Result<SiftBatch> {
        let limit = limit.clamp(1, 200);
        let ids: Vec<i64> = {
            let mut stmt = self.conn.prepare(
                "SELECT id FROM cards WHERE deck_id = ? AND state = 0 AND suspended = 0
                 AND position >= 0 ORDER BY position, id LIMIT ?",
            )?;
            let rows = stmt.query_map(params![deck_id, limit + exclude.len() as i64], |r| r.get(0))?;
            rows.collect::<std::result::Result<Vec<i64>, _>>()?
        };
        let cards = ids
            .into_iter()
            .filter(|id| !exclude.contains(id))
            .take(limit as usize)
            .map(|id| self.card_view(id))
            .collect::<Result<Vec<_>>>()?;
        let (remaining, total_new, known): (i64, i64, i64) = self.conn.query_row(
            "SELECT COALESCE(SUM(state = 0 AND suspended = 0 AND position >= 0), 0),
                    COALESCE(SUM(state = 0 AND suspended = 0), 0),
                    COALESCE(SUM(suspended = 2), 0)
             FROM cards WHERE deck_id = ?",
            [deck_id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        Ok(SiftBatch {
            cards,
            remaining,
            total_new,
            known,
        })
    }

    /// 不认识：保留为新词，并排到学习队列最前面（按过词的先后顺序）
    pub fn sift_keep(&mut self, card_id: i64) -> Result<()> {
        let deck_id: i64 =
            self.conn
                .query_row("SELECT deck_id FROM cards WHERE id = ?", [card_id], |r| r.get(0))?;
        let next: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(position), ?) + 1 FROM cards WHERE deck_id = ? AND position < 0",
            params![KEEP_BASE - 1, deck_id],
            |r| r.get(0),
        )?;
        self.conn.execute(
            "UPDATE cards SET position = ? WHERE id = ? AND state = 0",
            params![next.min(-1), card_id],
        )?;
        Ok(())
    }

    /// 认识：标为熟词
    pub fn sift_known(&mut self, card_id: i64) -> Result<()> {
        self.set_known(&[card_id], true)
    }

    /// 撤销一次过词操作：恢复为未过的新词
    pub fn sift_undo(&mut self, card_id: i64, position: i64) -> Result<()> {
        self.conn.execute(
            "UPDATE cards SET suspended = 0, position = ? WHERE id = ?",
            params![position, card_id],
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::models::SearchQuery;
    use crate::test_collection;

    #[test]
    fn sift_flow() {
        let (mut col, _d) = test_collection();
        let deck = col.create_deck("A").unwrap();
        for i in 0..6 {
            col.add_basic_note(deck, &format!("w{i}"), "x", "").unwrap();
        }
        let b = col.sift_batch(deck, 3, &[]).unwrap();
        assert_eq!(b.cards.len(), 3);
        assert_eq!((b.remaining, b.total_new, b.known), (6, 6, 0));
        let ids: Vec<i64> = b.cards.iter().map(|c| c.card_id).collect();

        // w0 认识，w1 不认识，w2 认识
        col.sift_known(ids[0]).unwrap();
        col.sift_keep(ids[1]).unwrap();
        col.sift_known(ids[2]).unwrap();
        let b = col.sift_batch(deck, 10, &[]).unwrap();
        assert_eq!((b.remaining, b.total_new, b.known), (3, 4, 2));
        assert_eq!(b.cards[0].fields[0], "w3");

        // 熟词不进入学习；不认识的 w1 排在最前面
        let st = col.study_state(deck).unwrap();
        assert_eq!(st.counts.new, 4);
        assert_eq!(st.card.unwrap().fields[0], "w1");
        let decks = col.list_decks().unwrap();
        assert_eq!(decks[0].known_cards, 2);
        assert_eq!(decks[0].new_cards, 4);

        // 熟词在单词本里可以筛选出来，并能取消
        let q = SearchQuery { state: "known".into(), ..Default::default() };
        assert_eq!(col.search_cards(&q).unwrap().total, 2);
        let all = col.search_card_ids(&q).unwrap();
        col.set_known(&all, false).unwrap();
        assert_eq!(col.search_cards(&q).unwrap().total, 0);

        // 撤销
        col.sift_undo(ids[1], 1).unwrap();
        assert_eq!(col.sift_batch(deck, 10, &[]).unwrap().remaining, 6);

        // 新添加的单词仍然算作「未过」
        col.sift_keep(ids[1]).unwrap();
        col.add_basic_note(deck, "new", "x", "").unwrap();
        let b = col.sift_batch(deck, 10, &[]).unwrap();
        assert_eq!(b.remaining, 6);
    }
}
