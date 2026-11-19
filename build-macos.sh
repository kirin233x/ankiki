#!/usr/bin/env bash
# =====================================================================
#  Ankiki macOS 一键打包脚本：生成可以发给别人的 .dmg
#
#  用法：
#    ./build-macos.sh            打包 Universal 版（Apple 芯片 + Intel 都能用）
#    ./build-macos.sh --native   只打包本机架构（更快，适合自己用）
#    ./build-macos.sh --help     查看帮助
#
#  产物：release/Ankiki_<版本>_<架构>.dmg
# =====================================================================
set -euo pipefail

cd "$(dirname "$0")"
ROOT="$(pwd)"
MODE="universal"

if [ -t 1 ]; then
  BOLD=$'\033[1m'; DIM=$'\033[2m'; RED=$'\033[31m'; GREEN=$'\033[32m'; PURPLE=$'\033[35m'; RESET=$'\033[0m'
else
  BOLD=""; DIM=""; RED=""; GREEN=""; PURPLE=""; RESET=""
fi
STEP=0
step() { STEP=$((STEP + 1)); echo; echo "${PURPLE}${BOLD}[$STEP] $*${RESET}"; }
info() { echo "    $*"; }
ok()   { echo "    ${GREEN}✓${RESET} $*"; }
die()  { echo; echo "${RED}${BOLD}✗ $*${RESET}" >&2; exit 1; }

while [ $# -gt 0 ]; do
  case "$1" in
    --native) MODE="native" ;;
    -h|--help) sed -n '3,10p' "$0" | sed 's/^#  //; s/^#//'; exit 0 ;;
    *) die "不认识的参数：$1（用 --help 查看用法）" ;;
  esac
  shift
done

START=$(date +%s)
echo "${BOLD}Ankiki · macOS 打包${RESET}  ${DIM}($MODE)${RESET}"

# ---------------------------------------------------------------- 1. 环境
step "检查构建环境"
[ "$(uname -s)" = "Darwin" ] || die "只能在 macOS 上打包"
xcode-select -p >/dev/null 2>&1 || die "缺少 Xcode 命令行工具，请先运行：xcode-select --install"

for p in /opt/homebrew/bin /usr/local/bin "$HOME/.cargo/bin"; do
  case ":$PATH:" in *":$p:"*) ;; *) [ -d "$p" ] && PATH="$p:$PATH" ;; esac
done
export PATH

command -v node  >/dev/null || die "没有找到 node，请先安装 Node.js（例如 brew install node）"
command -v cargo >/dev/null || die "没有找到 Rust，请先安装：curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
ok "Node $(node -v) · $(cargo --version)"

if [ "$MODE" = "universal" ]; then
  TARGET="universal-apple-darwin"
  ARCH_LABEL="universal"
  command -v rustup >/dev/null || die "Universal 打包需要 rustup（或者用 --native 只打本机架构）"
  for t in aarch64-apple-darwin x86_64-apple-darwin; do
    if ! rustup target list --installed | grep -q "^$t\$"; then
      info "安装编译目标 $t …"
      rustup target add "$t"
    fi
  done
  ok "编译目标：Apple 芯片 + Intel"
else
  TARGET=""
  case "$(uname -m)" in arm64) ARCH_LABEL="aarch64" ;; *) ARCH_LABEL="x64" ;; esac
  ok "编译目标：本机（$ARCH_LABEL）"
fi

# ---------------------------------------------------------------- 2. 依赖
step "安装前端依赖"
if [ ! -d node_modules ] || [ package-lock.json -nt node_modules/.package-lock.json ]; then
  npm install
  ok "依赖已安装"
else
  ok "依赖已是最新，跳过"
fi

# ---------------------------------------------------------------- 3. 打包
step "编译并打包（首次大约 5~10 分钟，Universal 会编译两遍）"
VERSION="$(node -p 'require("./src-tauri/tauri.conf.json").version')"
if [ -n "$TARGET" ]; then
  BUNDLE_DIR="$ROOT/src-tauri/target/$TARGET/release/bundle"
  rm -rf "$BUNDLE_DIR/dmg"
  npx tauri build --target "$TARGET" --bundles app,dmg
else
  BUNDLE_DIR="$ROOT/src-tauri/target/release/bundle"
  rm -rf "$BUNDLE_DIR/dmg"
  npx tauri build --bundles app,dmg
fi

DMG="$(ls -t "$BUNDLE_DIR"/dmg/*.dmg 2>/dev/null | head -1 || true)"
[ -n "$DMG" ] || die "打包完成了，但没有找到 .dmg，请把上面的输出发给我"

# ---------------------------------------------------------------- 4. 整理产物
step "整理安装包"
mkdir -p release
OUT="release/Ankiki_${VERSION}_${ARCH_LABEL}.dmg"
cp -f "$DMG" "$OUT"
SIZE="$(du -h "$OUT" | awk '{print $1}')"
ok "$OUT（$SIZE）"

if codesign -dv "$BUNDLE_DIR/macos/Ankiki.app" 2>&1 | grep -q 'Signature=adhoc'; then
  info "${DIM}签名：ad-hoc（未经 Apple 公证，别人第一次打开需要手动允许，见 README）${RESET}"
fi

if [ -t 1 ]; then open -R "$OUT" 2>/dev/null || true; fi

echo
echo "${GREEN}${BOLD}完成！${RESET} 用时 $(( $(date +%s) - START )) 秒"
echo "把 ${BOLD}$OUT${RESET} 发给别人即可。"
