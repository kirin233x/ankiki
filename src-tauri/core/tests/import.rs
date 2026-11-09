//! 用真实的 Anki 导出文件测试导入流程。
//! fixtures/legacy.apkg：genanki 生成的 collection.anki2（旧格式，JSON 媒体清单）
//! fixtures/modern.apkg：官方 Anki 导出的 collection.anki21b（zstd + protobuf），含学习进度

use ankiki_core::importer::ImportOptions;
use ankiki_core::models::*;
use ankiki_core::Collection;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn open() -> (Collection, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    (Collection::open(dir.path()).unwrap(), dir)
}

#[test]
fn import_legacy_apkg() {
    let (mut col, _d) = open();
    let res = col
        .import_package(&fixture("legacy.apkg"), &ImportOptions::default(), &|_| {})
        .unwrap();
    assert_eq!(res.deck_name, "大学英语四级");
    assert_eq!(res.notes_added, 6);
    // 5 个普通单词 + 1 个含两个填空的 cloze 笔记
    assert_eq!(res.cards_added, 7);
    assert_eq!(res.media_files, 2);

    let decks = col.list_decks().unwrap();
    assert_eq!(decks.len(), 1);
    assert_eq!(decks[0].due.new, 7);

    let st = col.study_state(res.deck_id).unwrap();
    let card = st.card.unwrap();
    assert_eq!(card.fields[0], "abandon");
    assert_eq!(card.notetype.fields[0], "Word");
    assert!(card.notetype.templates[0].qfmt.contains("{{Word}}"));
    assert!(card.notetype.css.contains("Georgia"));
    let media = PathBuf::from(&card.notetype.media_dir);
    assert!(media.join("abandon.wav").exists());
    assert!(media.join("pic.png").exists());

    // 再次导入：全部重复
    let err = col
        .import_package(&fixture("legacy.apkg"), &ImportOptions::default(), &|_| {})
        .unwrap_err();
    assert!(err.to_string().contains("全部导入过"));
    assert_eq!(col.list_decks().unwrap().len(), 1);

    // 删除词书后媒体目录一并清理
    col.delete_deck(res.deck_id).unwrap();
    assert!(!media.exists());
}

#[test]
fn import_modern_apkg_with_progress() {
    let (mut col, _d) = open();
    let res = col
        .import_package(&fixture("modern.apkg"), &ImportOptions::default(), &|_| {})
        .unwrap();
    assert_eq!(res.deck_name, "雅思核心词汇");
    assert_eq!(res.notes_added, 7);
    // Basic (and reversed)：每个单词两张卡
    assert_eq!(res.cards_added, 14);
    assert_eq!(res.media_files, 1);
    assert_eq!(res.with_progress, 4);

    let r = col
        .search_cards(&SearchQuery {
            state: "review".into(),
            ..Default::default()
        })
        .unwrap();
    assert_eq!(r.total, 4);
    assert!(r.rows.iter().all(|c| c.interval == 12));
    // 词书创建日（crt = 1793764800）之后第 5 天到期
    for row in &r.rows {
        assert_eq!(row.due, 1_793_764_800 + 5 * 86400);
    }

    let st = col.study_state(res.deck_id).unwrap();
    let card = st.card.unwrap();
    assert_eq!(card.notetype.templates.len(), 2);
    assert!(card.notetype.templates[0].qfmt.contains("{{Front}}"));
    assert_eq!(card.notetype.fields, vec!["Front", "Back"]);
    assert!(PathBuf::from(&card.notetype.media_dir)
        .join("ielts.wav")
        .exists());
}

#[test]
fn import_without_progress() {
    let (mut col, _d) = open();
    let opts = ImportOptions {
        include_progress: false,
        deck_name: Some("我的雅思".into()),
    };
    let res = col
        .import_package(&fixture("modern.apkg"), &opts, &|_| {})
        .unwrap();
    assert_eq!(res.deck_name, "我的雅思");
    assert_eq!(res.with_progress, 0);
    let decks = col.list_decks().unwrap();
    assert_eq!(decks[0].new_cards, 14);
}

#[test]
fn reject_non_package() {
    let (mut col, d) = open();
    let bad = d.path().join("bad.apkg");
    std::fs::write(&bad, b"not a zip").unwrap();
    assert!(col
        .import_package(&bad, &ImportOptions::default(), &|_| {})
        .is_err());
}

#[test]
fn deck_with_index_field_uses_real_word() {
    // 排序字段是「序号」(1, 2, 3)，单词在「拼写」字段
    let (mut col, _d) = open();
    let res = col
        .import_package(&fixture("indexed.apkg"), &ImportOptions::default(), &|_| {})
        .unwrap();
    let r = col.search_cards(&SearchQuery { deck_id: Some(res.deck_id), sort: "sort".into(), ..Default::default() }).unwrap();
    let fronts: Vec<&str> = r.rows.iter().map(|x| x.front.as_str()).collect();
    assert_eq!(fronts, vec!["abandon", "ability", "absorb"]);
    assert_eq!(r.rows[0].back, "v. 放弃");

    // 词表看板（全部已学需要先学一下）
    let st = col.study_state(res.deck_id).unwrap();
    col.answer_card(st.card.unwrap().card_id, 3, 1000).unwrap();
    let b = col.word_board(&BoardQuery::default()).unwrap();
    assert_eq!(b.items[0].word, "abandon");
    assert_eq!(b.items[0].meaning, "v. 放弃");

    // 手动指定单词字段
    let mut cfg = col.deck_config(res.deck_id).unwrap();
    cfg.word_field = "音标".into();
    col.update_deck_config(res.deck_id, cfg).unwrap();
    let r = col.search_cards(&SearchQuery { deck_id: Some(res.deck_id), sort: "sort".into(), ..Default::default() }).unwrap();
    assert_eq!(r.rows[0].front, "/əbˈzɔːb/");
}

#[test]
fn old_database_is_migrated_on_open() {
    let dir = tempfile::tempdir().unwrap();
    {
        let mut col = Collection::open(dir.path()).unwrap();
        col.import_package(&fixture("indexed.apkg"), &ImportOptions::default(), &|_| {})
            .unwrap();
        // 模拟旧版本：sort_field 存的是序号
        col.conn.execute("UPDATE notes SET sort_field = '1'", []).unwrap();
        col.conn.execute("DELETE FROM meta WHERE key = 'word_fields_v1'", []).unwrap();
    }
    let col = Collection::open(dir.path()).unwrap();
    let r = col.search_cards(&SearchQuery { sort: "sort".into(), ..Default::default() }).unwrap();
    assert_eq!(r.rows[0].front, "abandon");
}
