// 修复 Xcode 构建时找不到 npm 的问题（npm: command not found）。
//
// 从「访达」或 `open` 启动的 Xcode 只有系统默认的 PATH（/usr/bin:/bin…），
// 找不到 Homebrew / nvm 安装的 node、npm，也找不到 ~/.cargo/bin 下的 cargo。
// 这个脚本在 Xcode 工程的「Build Rust Code」脚本开头加一行 export PATH=…，
// 把当前终端里 node 所在的目录和常见的安装目录加进去。可以重复运行。
import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
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
