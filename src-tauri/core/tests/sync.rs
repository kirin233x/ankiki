//! 同步 Anki 学习进度的测试。
//! toefl-fresh.apkg：8 个单词的新词书（不含进度）
//! toefl-profile.anki2：同一本词书在 Anki 中学了 5 个单词后的本地数据库（含复习记录）
//! toefl-studied.apkg：同上，以「包含学习进度」方式导出

use ankiki_core::importer::ImportOptions;
use ankiki_core::models::*;
use ankiki_core::Collection;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

fn setup() -> (Collection, tempfile::TempDir, i64) {
    let dir = tempfile::tempdir().unwrap();
    let mut col = Collection::open(dir.path()).unwrap();
    let res = col
        .import_package(&fixture("toefl-fresh.apkg"), &ImportOptions::default(), &|_| {})
        .unwrap();
    assert_eq!(res.with_progress, 0);
    (col, dir, res.deck_id)
}

fn review_count(col: &Collection) -> i64 {
    col.search_cards(&SearchQuery { state: "review".into(), ..Default::default() })
        .unwrap()
        .total
}

#[test]
fn sync_from_anki_profile() {
    let (mut col, _d, deck) = setup();
    assert_eq!(review_count(&col), 0);

    let r = col.sync_from_anki(&fixture("toefl-profile.anki2"), &|_| {}).unwrap();
    assert_eq!(r.matched_notes, 8);
    assert_eq!(r.updated_cards, 5);
    assert_eq!(r.revlog_added, 5);
    assert_eq!(r.missing_notes, 0);
    assert_eq!(review_count(&col), 5);

    // 复习记录进入统计：今天学了 5 个新词
    let s = col.stats(Some(deck)).unwrap();
    assert_eq!(s.today_reviews, 5);
    assert_eq!(s.today_new, 5);
    // 今日新词额度相应减少
    let decks = col.list_decks().unwrap();
    assert_eq!(decks[0].due.new, 3);

    // 再同步一次：不会重复写入复习记录
    let r2 = col.sync_from_anki(&fixture("toefl-profile.anki2"), &|_| {}).unwrap();
    assert_eq!(r2.revlog_added, 0);
    assert_eq!(col.stats(Some(deck)).unwrap().today_reviews, 5);
}

#[test]
fn ankiki_newer_progress_is_kept() {
    let (mut col, _d, deck) = setup();
    col.sync_from_anki(&fixture("toefl-profile.anki2"), &|_| {}).unwrap();
    // 在 Ankiki 里接着复习一张已同步的卡（此时 last_review 比 Anki 更新）
    let r = col
        .search_cards(&SearchQuery { deck_id: Some(deck), state: "review".into(), ..Default::default() })
        .unwrap();
    let card = r.rows[0].card_id;
    std::thread::sleep(std::time::Duration::from_millis(1100));
    col.study_state(deck).unwrap();
    // 直接答题（复习卡未到期也可以作答）
    col.answer_card(card, 1, 1000).unwrap();
    let r2 = col.sync_from_anki(&fixture("toefl-profile.anki2"), &|_| {}).unwrap();
    assert_eq!(r2.skipped_newer, 1);
    assert_eq!(r2.updated_cards, 4);
    assert_eq!(col.card_detail(card).unwrap().row.lapses, 1);
}

#[test]
fn reimport_apkg_with_progress_updates_existing() {
    let (mut col, _d, deck) = setup();
    let res = col
        .import_package(&fixture("toefl-studied.apkg"), &ImportOptions::default(), &|_| {})
        .unwrap();
    assert_eq!(res.notes_added, 0);
    assert_eq!(res.progress_updated, 5);
    assert_eq!(res.deck_id, deck);
    assert_eq!(col.list_decks().unwrap().len(), 1);
    assert_eq!(review_count(&col), 5);
}

#[test]
fn sync_requires_imported_deck() {
    let dir = tempfile::tempdir().unwrap();
    let mut col = Collection::open(dir.path()).unwrap();
    let err = col.sync_from_anki(&fixture("toefl-profile.anki2"), &|_| {}).unwrap_err();
    assert!(err.to_string().contains("先把对应的词书导入"));
}
