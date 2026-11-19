// 对 `tauri ios init` 生成的 Xcode 工程做两处调整（可以重复运行）：
//
// 1. 修复 Xcode 构建时找不到 npm 的问题（npm: command not found）。
//    从「访达」或 `open` 启动的 Xcode 只有系统默认的 PATH（/usr/bin:/bin…），
//    找不到 Homebrew / nvm 安装的 node、npm，也找不到 ~/.cargo/bin 下的 cargo。
//    这里在「Build Rust Code」脚本开头加一行 export PATH=…。
//
// 2. 换上 Ankiki 的 App 图标和启动屏（纯品牌紫色），替换默认的白/黑色空白启动屏。
//    素材在 src-tauri/ios-assets/ 下。
import { cpSync, existsSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { homedir } from "node:os";

const appleDir = join(process.cwd(), "src-tauri", "gen", "apple");
if (!existsSync(appleDir)) {
  console.log("[ios-patch] 还没有生成 iOS 工程，跳过（先运行 npm run ios:init）");
  process.exit(0);
}
const proj = readdirSync(appleDir).find((f) => f.endsWith(".xcodeproj"));
if (!proj) {
  console.log("[ios-patch] 没有找到 .xcodeproj，跳过");
  process.exit(0);
}
// ---------------------------------------------------------------- 启动屏
const assetsSrc = join(process.cwd(), "src-tauri", "ios-assets");
if (existsSync(assetsSrc)) {
  cpSync(join(assetsSrc, "LaunchScreen.storyboard"), join(appleDir, "LaunchScreen.storyboard"));
  const catalog = join(appleDir, "Assets.xcassets");
  // 旧版本的启动 Logo 已不再使用
  rmSync(join(catalog, "LaunchLogo.imageset"), { recursive: true, force: true });
  for (const entry of readdirSync(join(assetsSrc, "Assets.xcassets"))) {
    // 整个替换（例如 AppIcon 原来有十几张不同尺寸的图，现在只用一张 1024 的）
    rmSync(join(catalog, entry), { recursive: true, force: true });
    cpSync(join(assetsSrc, "Assets.xcassets", entry), join(catalog, entry), { recursive: true });
  }
  console.log("[ios-patch] 已更新 App 图标和启动屏（AppIcon、LaunchScreen、LaunchBackground）");
}

// ---------------------------------------------------------------- PATH
const file = join(appleDir, proj, "project.pbxproj");
let text = readFileSync(file, "utf8");

const dirs = [
  dirname(process.execPath), // 当前使用的 node（nvm / Homebrew / 官方安装包都适用），npm 一般在同一目录
  join(homedir(), ".cargo", "bin"),
  "/opt/homebrew/bin",
  "/usr/local/bin",
];
const pathValue = [...new Set(dirs)].join(":");
// pbxproj 中字符串里的引号和换行需要转义成 \" 和 \n
const line = `export PATH=\\"${pathValue.replace(/"/g, '\\\\\\"')}:$PATH\\" # ankiki-path\\n`;

let patched = 0;
text = text.replace(/shellScript = "((?:[^"\\]|\\.)*)";/g, (m, body) => {
  if (!body.includes("xcode-script")) return m;
  const cleaned = body.replace(/export PATH=\\"[^\n]*?# ankiki-path\\n/g, "");
  patched++;
  return `shellScript = "${line}${cleaned}";`;
});

if (!patched) {
  console.log("[ios-patch] 没有找到「Build Rust Code」构建脚本，未做修改");
  process.exit(0);
}
writeFileSync(file, text);
console.log(`[ios-patch] 已为 Xcode 构建脚本补充 PATH：${pathValue}`);
