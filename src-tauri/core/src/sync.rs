//! 从本机 Anki 同步学习进度（单向：Anki → Ankiki）。
//!
//! 直接读取 Anki 的本地数据库 `collection.anki2`（先复制到临时目录，不会修改 Anki 的数据），
//! 按单词的 guid 与 Ankiki 中已导入的词书匹配，把较新的学习进度和复习记录同步过来。

use crate::importer::{read_collection, TempFile};
use crate::models::*;
use crate::util;
use crate::Collection;
use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};

/// Anki 数据目录在各系统上的默认位置
fn anki_base_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(home) = std::env::var("HOME") {
        let home = PathBuf::from(home);
        dirs.push(home.join("Library/Application Support/Anki2")); // macOS
        dirs.push(home.join(".local/share/Anki2")); // Linux
        dirs.push(home.join("Library/Application Support/Anki")); // 旧版 macOS
    }
    if let Ok(appdata) = std::env::var("APPDATA") {
        dirs.push(PathBuf::from(appdata).join("Anki2")); // Windows
    }
    dirs
}

/// 查找本机上所有 Anki 用户的数据库
pub fn find_anki_profiles() -> Vec<AnkiProfile> {
    let mut out = Vec::new();
    for base in anki_base_dirs() {
        let Ok(entries) = std::fs::read_dir(&base) else { continue };
        for entry in entries.flatten() {
            let dir = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name == "addons21" || name == "addons" || name.starts_with('.') {
                continue;
            }
            let db = dir.join("collection.anki2");
            let Ok(meta) = std::fs::metadata(&db) else { continue };
            // WAL 文件里可能有更新的数据，以两者中较新的修改时间为准
            let modified = [db.clone(), dir.join("collection.anki2-wal")]
                .iter()
                .filter_map(|p| std::fs::metadata(p).ok()?.modified().ok())
                .max()
                .or_else(|| meta.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            out.push(AnkiProfile {
                name,
                path: db.to_string_lossy().into_owned(),
                modified,
            });
        }
    }
    out.sort_by(|a, b| b.modified.cmp(&a.modified));
    out
}

/// 把 SQLite 数据库（连同 WAL 日志）复制到临时目录
fn copy_database(src: &Path) -> Result<TempFile> {
    let tmp = TempFile(std::env::temp_dir().join(format!(
        "ankiki-sync-{}-{:x}.anki2",
        util::now_millis(),
        rand::random::<u32>()
    )));
    std::fs::copy(src, &tmp.0).with_context(|| format!("无法读取 {}", src.display()))?;
    for ext in ["-wal", "-shm"] {
        let mut from = src.as_os_str().to_owned();
        from.push(ext);
        let from = PathBuf::from(from);
        if from.exists() {
            let mut to = tmp.0.as_os_str().to_owned();
            to.push(ext);
            let _ = std::fs::copy(&from, PathBuf::from(to));
        }
    }
    Ok(tmp)
}

impl Collection {
    /// 从 Anki 的 collection.anki2 同步学习进度
    pub fn sync_from_anki(
        &mut self,
        path: &Path,
        progress: &dyn Fn(ImportProgress),
    ) -> Result<SyncResult> {
        let report = |stage: &str| {
            progress(ImportProgress {
                stage: stage.into(),
                current: 0,
                total: 0,
            })
        };
        report("正在读取 Anki 数据…");
        let tmp = copy_database(path)?;
        let anki = read_collection(&tmp.0).context("无法读取 Anki 数据库，请确认选择的是 collection.anki2")?;
        if anki.notes.is_empty() {
            return Err(anyhow!("这个 Anki 数据库里没有任何单词"));
        }
        report("正在同步学习进度…");
        let up = self.apply_anki_progress(&anki)?;
        if up.matched_notes == 0 {
            return Err(anyhow!(
                "Anki 中的单词在 Ankiki 里都找不到。请先把对应的词书导入 Ankiki，再同步学习进度。"
            ));
        }
        Ok(SyncResult {
            matched_notes: up.matched_notes,
            updated_cards: up.updated_cards,
            skipped_newer: up.skipped_newer,
            revlog_added: up.revlog_added,
            missing_notes: up.missing_notes,
        })
    }
}
