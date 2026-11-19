# 各平台打包

Ankiki 所有平台共用同一套代码：

| 共用部分 | 位置 |
| --- | --- |
| 界面（React，桌面布局 + 手机玻璃风格布局） | `src/` |
| 核心逻辑（SQLite 数据库、FSRS 调度、Anki 导入、统计） | `src-tauri/core/` |
| 应用外壳（Tauri 命令、启动流程） | `src-tauri/src/` |
| 内置词书 | `src-tauri/resources/` |

`platforms/` 下每个子目录只放**该平台专属**的打包脚本和素材。脚本都可以在任意目录下运行，会自动切换到项目根目录。

| 平台 | 打包命令 | 产物 | 状态 |
| --- | --- | --- | --- |
| macOS | `platforms/macos/build.sh` | `release/Ankiki_<版本>_universal.dmg` | ✅ 可用，CI 自动打包 arm64 |
| Windows | 双击 `platforms\windows\build.cmd` | `release\Ankiki_<版本>_x64-setup.exe` | ✅ 可用，CI 自动打包 |
| iOS | `platforms/ios/build.sh` | 直接安装到连接的 iPhone | ✅ 可用（需要在 Mac 上，用自己的签名） |
| Android | 见 [android/README.md](android/README.md) | `.apk` | 🚧 规划中 |
| 鸿蒙 | 见 [harmony/README.md](harmony/README.md) | `.hap` | 🚧 规划中 |

GitHub Actions（`.github/workflows/build.yml`）会在推送到 master 时自动打包 macOS 和 Windows；`src-tauri/tauri.conf.json` 里的版本号还没发布过时，会自动发布到 Releases。
