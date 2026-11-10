use ankiki_core::importer::ImportOptions;
use ankiki_core::models::*;
use ankiki_core::Collection;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, State};

struct AppState(Mutex<Collection>);

type CmdResult<T> = Result<T, String>;

fn err(e: anyhow::Error) -> String {
    format!("{e}")
}

macro_rules! with_col {
    ($state:expr, $col:ident => $body:expr) => {{
        let mut guard = $state.0.lock().map_err(|_| "数据库被占用，请重试".to_string())?;
        #[allow(unused_mut)]
        let mut $col = &mut *guard;
        $body.map_err(err)
    }};
}

// ------------------------------------------------------------------ 设置

#[tauri::command]
fn get_settings(state: State<AppState>) -> CmdResult<Settings> {
    with_col!(state, col => Ok::<_, anyhow::Error>(col.settings().clone()))
}

#[tauri::command]
fn save_settings(state: State<AppState>, settings: Settings) -> CmdResult<()> {
    with_col!(state, col => col.save_settings(settings))
}

// ------------------------------------------------------------------ 词书

#[tauri::command]
fn list_decks(state: State<AppState>) -> CmdResult<Vec<DeckInfo>> {
    with_col!(state, col => col.list_decks())
}

#[tauri::command]
fn create_deck(state: State<AppState>, name: String) -> CmdResult<i64> {
    with_col!(state, col => col.create_deck(&name))
}

#[tauri::command]
fn rename_deck(state: State<AppState>, deck_id: i64, name: String) -> CmdResult<()> {
    with_col!(state, col => col.rename_deck(deck_id, &name))
}

#[tauri::command]
fn set_deck_description(state: State<AppState>, deck_id: i64, description: String) -> CmdResult<()> {
    with_col!(state, col => col.set_deck_description(deck_id, &description))
}

#[tauri::command]
fn delete_deck(state: State<AppState>, deck_id: i64) -> CmdResult<()> {
    with_col!(state, col => col.delete_deck(deck_id))
}

#[tauri::command]
fn update_deck_config(state: State<AppState>, deck_id: i64, config: DeckConfig) -> CmdResult<()> {
    with_col!(state, col => col.update_deck_config(deck_id, config))
}

// ------------------------------------------------------------------ 学习

#[tauri::command]
fn study_state(state: State<AppState>, deck_id: i64) -> CmdResult<StudyState> {
    with_col!(state, col => col.study_state(deck_id))
}

#[tauri::command]
fn answer_card(
    state: State<AppState>,
    card_id: i64,
    rating: u8,
    duration_ms: i64,
) -> CmdResult<StudyState> {
    with_col!(state, col => col.answer_card(card_id, rating, duration_ms))
}

#[tauri::command]
fn undo(state: State<AppState>) -> CmdResult<Option<StudyState>> {
    with_col!(state, col => col.undo())
}

// ------------------------------------------------------------------ 导入

/// 把文件选择器返回的路径转成本地路径。
/// iOS 上返回的是 `file:///private/var/.../xxx.apkg` 形式（含百分号编码）
fn local_path(path: &str) -> PathBuf {
    let Some(rest) = path.strip_prefix("file://") else {
        return PathBuf::from(path);
    };
    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
    let bytes = rest.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(b) = u8::from_str_radix(&rest[i + 1..i + 3], 16) {
                out.push(b);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    PathBuf::from(String::from_utf8_lossy(&out).into_owned())
}

#[tauri::command]
async fn import_package(
    app: AppHandle,
    path: String,
    options: Option<ImportOptions>,
) -> CmdResult<ImportResult> {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = handle.state::<AppState>();
        let mut col = state
            .0
            .lock()
            .map_err(|_| "数据库被占用，请重试".to_string())?;
        let emitter = handle.clone();
        col.import_package(
            &local_path(&path),
            &options.unwrap_or_default(),
            &move |p| {
                let _ = emitter.emit("import-progress", p);
            },
        )
        .map_err(err)
    })
    .await
    .map_err(|e| format!("导入失败：{e}"))?
}

#[tauri::command]
fn find_anki_profiles() -> Vec<AnkiProfile> {
    ankiki_core::sync::find_anki_profiles()
}

#[tauri::command]
async fn sync_from_anki(app: AppHandle, path: String) -> CmdResult<SyncResult> {
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let state = handle.state::<AppState>();
        let mut col = state
            .0
            .lock()
            .map_err(|_| "数据库被占用，请重试".to_string())?;
        let emitter = handle.clone();
        col.sync_from_anki(&PathBuf::from(path), &move |p| {
            let _ = emitter.emit("import-progress", p);
        })
        .map_err(err)
    })
    .await
    .map_err(|e| format!("同步失败：{e}"))?
}

// ------------------------------------------------------------------ 浏览 / 编辑

#[tauri::command]
fn search_cards(state: State<AppState>, query: SearchQuery) -> CmdResult<SearchResult> {
    with_col!(state, col => col.search_cards(&query))
}

#[tauri::command]
fn card_detail(state: State<AppState>, card_id: i64) -> CmdResult<CardDetail> {
    with_col!(state, col => col.card_detail(card_id))
}

#[tauri::command]
fn add_note(
    state: State<AppState>,
    deck_id: i64,
    front: String,
    back: String,
    tags: Option<String>,
) -> CmdResult<i64> {
    with_col!(state, col => col.add_basic_note(deck_id, &front, &back, tags.as_deref().unwrap_or("")))
}

#[tauri::command]
fn update_note(
    state: State<AppState>,
    note_id: i64,
    fields: Vec<String>,
    tags: String,
) -> CmdResult<()> {
    with_col!(state, col => col.update_note(note_id, fields, &tags))
}

#[tauri::command]
fn delete_notes(state: State<AppState>, note_ids: Vec<i64>) -> CmdResult<()> {
    with_col!(state, col => col.delete_notes(&note_ids))
}

#[tauri::command]
fn set_suspended(state: State<AppState>, card_ids: Vec<i64>, suspended: bool) -> CmdResult<()> {
    with_col!(state, col => col.set_suspended(&card_ids, suspended))
}

#[tauri::command]
fn set_known(state: State<AppState>, card_ids: Vec<i64>, known: bool) -> CmdResult<()> {
    with_col!(state, col => col.set_known(&card_ids, known))
}

#[tauri::command]
fn search_card_ids(state: State<AppState>, query: SearchQuery) -> CmdResult<Vec<i64>> {
    with_col!(state, col => col.search_card_ids(&query))
}

#[tauri::command]
fn delete_cards(state: State<AppState>, card_ids: Vec<i64>) -> CmdResult<()> {
    with_col!(state, col => col.delete_notes_of_cards(&card_ids))
}

#[tauri::command]
fn sift_batch(
    state: State<AppState>,
    deck_id: i64,
    limit: i64,
    exclude: Vec<i64>,
) -> CmdResult<SiftBatch> {
    with_col!(state, col => col.sift_batch(deck_id, limit, &exclude))
}

#[tauri::command]
fn sift_answer(state: State<AppState>, card_id: i64, known: bool) -> CmdResult<()> {
    with_col!(state, col => if known { col.sift_known(card_id) } else { col.sift_keep(card_id) })
}

#[tauri::command]
fn sift_undo(state: State<AppState>, card_id: i64, position: i64) -> CmdResult<()> {
    with_col!(state, col => col.sift_undo(card_id, position))
}

#[tauri::command]
fn forget_cards(state: State<AppState>, card_ids: Vec<i64>) -> CmdResult<()> {
    with_col!(state, col => col.forget_cards(&card_ids))
}

// ------------------------------------------------------------------ 统计

#[tauri::command]
fn get_stats(state: State<AppState>, deck_id: Option<i64>) -> CmdResult<Stats> {
    with_col!(state, col => col.stats(deck_id))
}

#[tauri::command]
fn word_board(state: State<AppState>, query: BoardQuery) -> CmdResult<BoardResult> {
    with_col!(state, col => col.word_board(&query))
}

#[tauri::command]
fn deck_fields(state: State<AppState>, deck_id: i64) -> CmdResult<Vec<String>> {
    with_col!(state, col => col.deck_fields(deck_id))
}

#[tauri::command]
fn data_dir(app: AppHandle) -> CmdResult<String> {
    app.path()
        .app_data_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            let col = Collection::open(&dir)
                .map_err(|e| format!("无法打开数据目录 {}：{e}", dir.display()))?;
            app.manage(AppState(Mutex::new(col)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_settings,
            save_settings,
            list_decks,
            create_deck,
            rename_deck,
            set_deck_description,
            delete_deck,
            update_deck_config,
            study_state,
            answer_card,
            undo,
            import_package,
            find_anki_profiles,
            sync_from_anki,
            search_cards,
            card_detail,
            add_note,
            update_note,
            delete_notes,
            set_suspended,
            forget_cards,
            set_known,
            search_card_ids,
            delete_cards,
            sift_batch,
            sift_answer,
            sift_undo,
            get_stats,
            word_board,
            deck_fields,
            data_dir,
        ])
        .run(tauri::generate_context!())
        .expect("启动 Ankiki 失败");
}

#[cfg(test)]
mod tests {
    use super::local_path;
    use std::path::PathBuf;

    #[test]
    fn file_urls() {
        assert_eq!(local_path("/a/b.apkg"), PathBuf::from("/a/b.apkg"));
        assert_eq!(
            local_path("file:///private/var/tmp/%E5%9B%9B%E7%BA%A7%20words.apkg"),
            PathBuf::from("/private/var/tmp/四级 words.apkg")
        );
    }
}
