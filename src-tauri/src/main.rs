// 在 Windows 发布版中隐藏控制台窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    ankiki_lib::run()
}
