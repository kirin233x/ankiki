# Ankiki

一个仿照 Anki 的**轻量、好看**的背单词应用。基于 [Tauri 2](https://v2.tauri.app/)（Rust + 系统 WebView），安装包体积小、启动快，不会卡顿。

## 功能

- **导入 Anki 词书**：支持 `.apkg` / `.colpkg`，包括 Anki 2.0、2.1 以及 2.1.50+ 的新格式（zstd + protobuf）。卡片模板、样式、发音、图片都会保留，也可以选择导入原有的学习进度。拖进窗口就能导入。
- **从 Anki 同步进度**：直接读取本机 Anki 的数据（只读，不改动 Anki），把已经背过的单词进度和复习记录同步到 Ankiki 里的同一本词书，可以反复同步；两边都学过的卡片，以最近一次复习的一方为准。
- **快速过词 / 熟词**：把一本词书的新词快速过一遍，认识的按 `1` 标为熟词，从每日任务和复习里彻底排除；不认识的按 `2` 排到学习队列最前面。学习时按 `K` 也能把太简单的词标为熟词。熟词可以在单词本里随时恢复。
- **FSRS 智能复习**：使用 FSRS-5 记忆算法，有四个评分按钮（重来 / 困难 / 良好 / 简单），按钮上会显示下一次复习的时间。
- **每日计划**：每本词书可以单独设置每天的新词数、复习上限、目标记忆保持率和学习步长。
- **单词本**：浏览、搜索、筛选全部单词，可以查看每张卡片的记忆状态和复习记录，也可以编辑、暂停、重置或删除。支持多选（Shift 连选），或者一键全选筛选结果后批量操作。
- **词表看板**：一页浏览某一天、近 7 天或全部已学的单词，英文和中文释义一起显示。重来/困难标红，良好标绿，简单标蓝，新学的词带「新」标记。可以按颜色筛选，也可以隐藏释义自测（快捷键 H，← → 切换日期）。在学习统计的日历上点某一天也能直接跳过来。
- **学习统计**：包括学习日历热力图、未来 30 天复习预测、卡片分布、间隔分布和评分分布。
- **其他**：浅色 / 深色主题、撤销评分、自动播放发音、手动添加单词。

### 快捷键（学习界面）

| 按键 | 作用 |
| --- | --- |
| 空格 / 回车 | 显示答案；翻面后等同「良好」 |
| 1 2 3 4 | 重来 / 困难 / 良好 / 简单 |
| Z 或 ⌘Z | 撤销上一次评分 |
| R | 重播发音 |
| K | 太简单，标为熟词 |
| E | 编辑当前卡片 |
| Esc | 退出学习 |

## 在 Mac 上编译

只需要准备一次环境：

```bash
# 1. Xcode 命令行工具
xcode-select --install

# 2. Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 3. Node.js 18+（推荐用 Homebrew）
brew install node
```

然后在项目目录下运行：

```bash
npm install
npm run tauri build
```

第一次编译大约需要 5~10 分钟。完成后会生成：

- `src-tauri/target/release/bundle/macos/Ankiki.app`：把它拖进「应用程序」文件夹就能使用。
- `src-tauri/target/release/bundle/dmg/Ankiki_0.1.0_*.dmg`：安装包。

开发调试（修改代码后自动刷新）：

```bash
npm run tauri dev
```

> 不想自己编译的话，可以在 GitHub 仓库的 **Actions → 打包 macOS 应用** 中手动运行，然后下载生成的 DMG。从网上下载的未签名应用第一次打开时，macOS 可能提示「已损坏」，在终端执行 `xattr -cr /Applications/Ankiki.app` 即可。

## iPhone 版（实验性）

手机版和 Mac 版共用同一套代码，界面会自动切换成手机布局：底部是悬浮的玻璃标签栏，学习时轻点卡片翻面。手机和 Mac 的数据各自独立，暂时不同步。

### 第一次准备（只需一次）

1. 从 App Store 安装 **Xcode**，打开一次并同意许可协议。
2. 安装工具：

```bash
brew install cocoapods
```

```bash
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
```

3. 在项目目录下生成 iOS 工程：

```bash
npm install
```

```bash
npm run ios:init
```

4. 用数据线把 iPhone 连到 Mac。在 iPhone 上打开「设置 → 隐私与安全性 → 开发者模式」（如果看不到这一项，先在 Xcode 里连一次手机）。

### 安装到手机

1. **先彻底退出 Xcode（⌘Q）**。如果 Xcode 已经开着，新打开的工程会沿用旧的环境，找不到 `npm`。
2. 在终端里运行下面的命令，**运行期间不要关掉这个终端**：

```bash
npm run ios:xcode
```

这条命令会做三件事：
- 给 Xcode 的构建脚本补上 node 和 cargo 的路径，避免 `npm: command not found`；
- 构建界面；
- 由 Tauri 打开 Xcode，并一直在后台运行。Xcode 构建 Rust 代码时要从它读取参数。

3. 在 Xcode 里：左侧点项目 → 选中 `ankiki_iOS` → **Signing & Capabilities** → Team 选你的 Apple ID（Personal Team 也可以）。如果提示 Bundle Identifier 冲突，改成别的，比如 `com.你的名字.ankiki`。
4. 顶部选择你的 iPhone，点 ▶（⌘R）。界面已经打包进 App，装好后不依赖 Mac 也能用。
5. 第一次打开时，如果提示「未受信任的开发者」，在 iPhone 上打开「设置 → 通用 → VPN 与设备管理」，信任你的开发者证书。
6. 装好后可以在终端按 Ctrl+C 结束。

> 用免费 Apple ID 签名的 App **7 天后会过期**。过期后重复上面的步骤即可，词书和学习进度不会丢。

> 启动画面：`npm run ios:xcode` 会自动给 Xcode 工程换上 Ankiki 的启动屏（紫色 + Logo），素材在 `src-tauri/ios-assets/`。iOS 会缓存启动屏，如果装好后还看到旧的黑屏，重启一下手机即可。

**常见报错**

- `npm: command not found`：先 ⌘Q 退出 Xcode，再运行 `npm run ios:xcode`，它会自动修复。如果换了 node 版本（比如用 nvm 切换），也重新运行一次。
- `the tauri ios dev or tauri ios build command must be running`：运行 `npm run ios:xcode` 的终端被关掉了，重新运行即可。

开发调试时也可以用 `npm run ios:dev`（同样会打开 Xcode）。这种方式下，App 从 Mac 上的开发服务器加载界面，手机和 Mac 需要连在同一个 Wi-Fi。

### 在手机上导入词书

先把 .apkg 放进 iPhone 的「文件」App（例如从 Mac 用隔空投送发到手机后选择「存储到文件」，或者放进 iCloud 云盘），然后在 App 里点右下角的 **+** → 「导入 Anki 词书」，选中文件即可。

在电脑浏览器里预览手机布局：运行 `npm run dev`，然后打开 `http://localhost:1420/?mobile`。

## 数据存放位置

所有数据都保存在本地，不联网：

```
~/Library/Application Support/com.kirin233x.ankiki/
├── ankiki.db     # 单词、卡片、复习记录（SQLite）
└── media/        # 词书中的音频与图片
```

需要备份时，复制这个文件夹即可。

## 从 Anki 迁移（保留背诵进度）

1. 先把词书导入 Ankiki（用原始词书或从 Anki 导出的 .apkg 都可以）。
2. 退出 Anki，确保最新进度已经保存。
3. 在 Ankiki 首页点「从 Anki 同步」（也可以在「设置 → Anki」里找到），选择你的 Anki 用户，然后点「开始同步」。

Ankiki 会自动在 `~/Library/Application Support/Anki2/` 下查找 Anki 用户；如果没找到，可以手动选择对应用户文件夹里的 `collection.anki2`。同步按单词的唯一标识（guid）匹配，Anki 中还没导入 Ankiki 的词书会被跳过。

另一种方式：在 Anki 导出 .apkg 时勾选「包含学习进度」，再导入 Ankiki。已经存在的单词也会更新为其中较新的进度。

## 导入词书的小提示

- 在 Anki 中选中词书，选择「导出」→「Anki 卡组包 (*.apkg)」，再导入到 Ankiki。勾选「包含学习进度」可以继续之前的复习计划。
- 如果遇到无法识别的词书，可以在 Anki 导出时勾选「支持旧版本 Anki」后再试。
- 重复导入同一本词书时，已经存在的单词会自动跳过。

## 项目结构

```
├── src/                     # 前端（React + TypeScript）
│   ├── pages/               # 首页、学习、单词本、统计、设置
│   ├── components/          # 卡片渲染 iframe、对话框等
│   ├── render.ts            # Anki 卡片模板渲染器（字段、填空、发音、注音…）
│   └── api.ts               # 调用 Rust 后端的接口
└── src-tauri/
    ├── src/lib.rs           # Tauri 命令
    └── core/                # 与界面无关的核心库（可单独测试）
        ├── src/importer.rs  # .apkg / .colpkg 导入
        ├── src/fsrs.rs      # FSRS-5 记忆模型
        ├── src/scheduler.rs # 学习队列与评分调度
        ├── src/browse.rs    # 搜索
        └── src/stats.rs     # 统计
```

运行核心测试：

```bash
cd src-tauri && cargo test -p ankiki-core
```
