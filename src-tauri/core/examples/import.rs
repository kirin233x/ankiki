//! 命令行导入工具（调试用）：
//! cargo run -p ankiki-core --example import -- <数据目录> <词书.apkg>
use ankiki_core::importer::ImportOptions;
use ankiki_core::Collection;
use std::path::PathBuf;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("用法: import <数据目录> <词书.apkg>...");
        std::process::exit(1);
    }
    let mut col = Collection::open(&PathBuf::from(&args[1]))?;
    for path in &args[2..] {
        let res = col.import_package(&PathBuf::from(path), &ImportOptions::default(), &|p| {
            if p.total > 0 {
                eprint!("\r{} {}/{}   ", p.stage, p.current, p.total);
            }
        })?;
        eprintln!();
        println!(
            "导入「{}」：{} 个单词，{} 张卡片，{} 个媒体文件，{} 张带进度",
            res.deck_name, res.notes_added, res.cards_added, res.media_files, res.with_progress
        );
    }
    Ok(())
}
