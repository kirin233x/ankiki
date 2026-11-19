#!/usr/bin/env bash
# =====================================================================
#  Ankiki iOS 一键构建脚本
#
#  用法：
#    ./build_ios.sh              构建 → 安装到连接的 iPhone → 自动打开 App
#    ./build_ios.sh --xcode      构建前端并打开 Xcode（在 Xcode 里手动点 ▶，适合调试）
#    ./build_ios.sh --device ID  指定设备（有多台设备时使用，ID 见 xcrun devicectl list devices）
#    ./build_ios.sh --help       查看帮助
#
#  第一次使用前：在 Xcode 里给工程选过一次 Team（Signing & Capabilities），
#  或者设置环境变量 APPLE_DEVELOPMENT_TEAM=你的 Team ID。
# =====================================================================
set -euo pipefail

cd "$(dirname "$0")"
ROOT="$(pwd)"
APPLE_DIR="$ROOT/src-tauri/gen/apple"

MODE="install"
DEVICE="${DEVICE:-}"

# ---------------------------------------------------------------- 输出样式
if [ -t 1 ]; then
  BOLD=$'\033[1m'; DIM=$'\033[2m'; RED=$'\033[31m'; GREEN=$'\033[32m'; YELLOW=$'\033[33m'; PURPLE=$'\033[35m'; RESET=$'\033[0m'
else
  BOLD=""; DIM=""; RED=""; GREEN=""; YELLOW=""; PURPLE=""; RESET=""
fi
STEP=0
step() { STEP=$((STEP + 1)); echo; echo "${PURPLE}${BOLD}[$STEP] $*${RESET}"; }
info() { echo "    $*"; }
ok()   { echo "    ${GREEN}✓${RESET} $*"; }
warn() { echo "    ${YELLOW}!${RESET} $*"; }
die()  { echo; echo "${RED}${BOLD}✗ $*${RESET}" >&2; exit 1; }

usage() {
  sed -n '3,12p' "$0" | sed 's/^#  //; s/^#//'
  exit 0
}

while [ $# -gt 0 ]; do
  case "$1" in
    --xcode) MODE="xcode" ;;
    --device) shift; DEVICE="${1:-}"; [ -n "$DEVICE" ] || die "--device 后面需要设备 ID" ;;
    -h|--help) usage ;;
    *) die "不认识的参数：$1（用 --help 查看用法）" ;;
  esac
  shift
done

START=$(date +%s)
echo "${BOLD}Ankiki · iOS 构建${RESET}  ${DIM}($MODE)${RESET}"

# ---------------------------------------------------------------- 1. 检查环境
step "检查构建环境"

[ "$(uname -s)" = "Darwin" ] || die "iOS 只能在 macOS 上构建"
command -v xcodebuild >/dev/null || die "没有找到 Xcode，请先从 App Store 安装 Xcode 并打开一次"
ok "Xcode $(xcodebuild -version 2>/dev/null | head -1 | awk '{print $2}')"

# 让 Homebrew、cargo 在脚本里可用（从 Finder / 其它工具启动时 PATH 可能不完整）
for p in /opt/homebrew/bin /usr/local/bin "$HOME/.cargo/bin"; do
  case ":$PATH:" in *":$p:"*) ;; *) [ -d "$p" ] && PATH="$p:$PATH" ;; esac
done
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
export PATH

command -v node >/dev/null || die "没有找到 node，请先安装 Node.js（例如 brew install node）"
command -v npm  >/dev/null || die "没有找到 npm"
ok "Node $(node -v)"

command -v cargo  >/dev/null || die "没有找到 Rust，请先安装：curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
command -v rustup >/dev/null || die "没有找到 rustup"
ok "$(cargo --version)"

if ! rustup target list --installed | grep -q '^aarch64-apple-ios$'; then
  info "安装 iOS 编译目标 aarch64-apple-ios …"
  rustup target add aarch64-apple-ios
fi
ok "Rust iOS 编译目标已就绪"

# ---------------------------------------------------------------- 2. 依赖
step "安装前端依赖"
if [ ! -d node_modules ] || [ package-lock.json -nt node_modules/.package-lock.json ]; then
  npm install
  ok "依赖已安装"
else
  ok "依赖已是最新，跳过"
fi

# ---------------------------------------------------------------- 3. Xcode 工程
step "准备 Xcode 工程"
if ! ls "$APPLE_DIR"/*.xcodeproj >/dev/null 2>&1; then
  info "第一次构建，生成 iOS 工程 …"
  if ! command -v xcodegen >/dev/null && command -v brew >/dev/null; then
    info "安装 xcodegen …"
    brew install xcodegen
  fi
  npx tauri ios init
fi
# 补 PATH、换上 App 图标和启动屏（可重复执行）
node scripts/ios-patch-xcode.mjs | sed 's/^/    /'

# 签名用的 Team：优先用环境变量，否则读取在 Xcode 里选过的 Team
if [ -z "${APPLE_DEVELOPMENT_TEAM:-}" ]; then
  APPLE_DEVELOPMENT_TEAM="$(grep -ho 'DEVELOPMENT_TEAM = [A-Z0-9]*;' "$APPLE_DIR"/*.xcodeproj/project.pbxproj 2>/dev/null \
    | head -1 | sed 's/DEVELOPMENT_TEAM = \(.*\);/\1/' || true)"
fi
if [ -n "$APPLE_DEVELOPMENT_TEAM" ]; then
  export APPLE_DEVELOPMENT_TEAM
  ok "签名 Team：$APPLE_DEVELOPMENT_TEAM"
elif [ "$MODE" = "install" ]; then
  die "还没有设置签名 Team。
  方法一：运行 ./build_ios.sh --xcode，在 Xcode 的 Signing & Capabilities 里选一次 Team，以后就不用再选了；
  方法二：export APPLE_DEVELOPMENT_TEAM=你的TeamID 后再运行本脚本。"
fi

# ---------------------------------------------------------------- Xcode 模式
if [ "$MODE" = "xcode" ]; then
  step "打开 Xcode"
  if pgrep -xq Xcode; then
    info "先退出正在运行的 Xcode，让新的工程设置生效 …"
    osascript -e 'tell application "Xcode" to quit' >/dev/null 2>&1 || true
    sleep 2
  fi
  info "Xcode 打开后：选择你的 iPhone，点 ▶ 运行。"
  info "${BOLD}构建过程中请不要关闭这个终端${RESET}（Xcode 需要从这里读取构建参数），装好后按 Ctrl+C 结束。"
  exec npx tauri ios build --open
fi

# ---------------------------------------------------------------- 4. 构建
step "构建 App（界面 + Rust，首次大约需要 5~10 分钟）"
STAMP="$(mktemp)"
trap 'rm -f "$STAMP"' EXIT
npx tauri ios build --archive-only

ARCHIVE="$(find "$APPLE_DIR" -name '*.xcarchive' -type d -newer "$STAMP" -prune 2>/dev/null | head -1)"
[ -n "$ARCHIVE" ] || ARCHIVE="$(ls -dt "$APPLE_DIR"/build/*.xcarchive 2>/dev/null | head -1 || true)"
[ -n "$ARCHIVE" ] || die "构建完成了，但没有找到 .xcarchive，请把上面的输出发给我"
APP="$(ls -d "$ARCHIVE"/Products/Applications/*.app 2>/dev/null | head -1 || true)"
[ -n "$APP" ] || die "没有在 $ARCHIVE 里找到 .app"
BUNDLE_ID="$(/usr/libexec/PlistBuddy -c 'Print CFBundleIdentifier' "$APP/Info.plist")"
ok "构建完成：$(basename "$APP")（$BUNDLE_ID）"

# ---------------------------------------------------------------- 5. 选择设备
step "查找 iPhone"
xcrun devicectl --version >/dev/null 2>&1 || die "需要 Xcode 15 或更高版本（缺少 xcrun devicectl）"

if [ -z "$DEVICE" ]; then
  DEVICES_JSON="$(mktemp)"
  xcrun devicectl list devices --json-output "$DEVICES_JSON" >/dev/null 2>&1 || true
  # 选第一台已配对的 iPhone / iPad；已连接的优先
  DEVICE_LINE="$(node -e '
    const fs = require("fs");
    let list = [];
    try { list = JSON.parse(fs.readFileSync(process.argv[1], "utf8")).result.devices || []; } catch {}
    const ios = list.filter((d) =>
      (d.hardwareProperties?.platform || "").toLowerCase() === "ios" &&
      d.connectionProperties?.pairingState === "paired" &&
      d.connectionProperties?.tunnelState !== "unavailable");
    ios.sort((a, b) => (a.connectionProperties.tunnelState === "connected" ? 0 : 1) - (b.connectionProperties.tunnelState === "connected" ? 0 : 1));
    if (ios[0]) console.log(ios[0].identifier + "\t" + (ios[0].deviceProperties?.name || "iPhone"));
  ' "$DEVICES_JSON")"
  rm -f "$DEVICES_JSON"
  [ -n "$DEVICE_LINE" ] || die "没有找到可用的 iPhone。
  请确认：① 手机用数据线连到 Mac 并已解锁；② 手机上点过「信任此电脑」；③ 已开启「设置 → 隐私与安全性 → 开发者模式」。
  也可以运行 ./build_ios.sh --xcode，用 Xcode 安装。"
  DEVICE="${DEVICE_LINE%%$'\t'*}"
  ok "设备：${DEVICE_LINE#*$'\t'}"
else
  ok "设备：$DEVICE"
fi

# ---------------------------------------------------------------- 6. 安装并启动
step "安装到手机"
if ! xcrun devicectl device install app --device "$DEVICE" "$APP"; then
  die "安装失败。常见原因：手机锁屏了、没开开发者模式、或证书过期（免费账号 7 天）。
  可以运行 ./build_ios.sh --xcode，用 Xcode 安装并查看详细原因。"
fi
ok "已安装"

step "启动 App"
if xcrun devicectl device process launch --terminate-existing --device "$DEVICE" "$BUNDLE_ID" >/dev/null 2>&1; then
  ok "已在手机上打开 Ankiki"
else
  warn "没能自动打开。如果是第一次安装，请在手机「设置 → 通用 → VPN 与设备管理」里信任你的开发者证书，然后手动打开 App。"
fi

echo
echo "${GREEN}${BOLD}完成！${RESET} 用时 $(( $(date +%s) - START )) 秒"
