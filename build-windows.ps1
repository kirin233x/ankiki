# =====================================================================
#  Ankiki Windows 一键打包脚本：生成可以发给别人的安装程序（.exe）
#
#  用法（在项目目录打开 PowerShell）：
#    .\build-windows.ps1
#  或者直接双击 build-windows.cmd
#
#  产物：release\Ankiki_<版本>_x64-setup.exe
#  需要：Node.js 18+、Rust（rustup）、Visual Studio 生成工具（勾选「使用 C++ 的桌面开发」）
# =====================================================================
$ErrorActionPreference = "Stop"
Set-Location -Path $PSScriptRoot
$start = Get-Date
$script:step = 0

function Step($msg) { $script:step++; Write-Host ""; Write-Host "[$script:step] $msg" -ForegroundColor Magenta }
function Ok($msg)   { Write-Host "    √ $msg" -ForegroundColor Green }
function Info($msg) { Write-Host "    $msg" }
function Die($msg)  { Write-Host ""; Write-Host "× $msg" -ForegroundColor Red; exit 1 }
function Has($cmd)  { [bool](Get-Command $cmd -ErrorAction SilentlyContinue) }

Write-Host "Ankiki · Windows 打包" -ForegroundColor White

# ---------------------------------------------------------------- 1. 环境
Step "检查构建环境"
$cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
if ((Test-Path $cargoBin) -and -not ($env:Path -split ";" -contains $cargoBin)) { $env:Path = "$cargoBin;$env:Path" }

if (-not (Has node))  { Die "没有找到 Node.js，请先安装：https://nodejs.org （或 winget install OpenJS.NodeJS.LTS）" }
if (-not (Has cargo)) { Die "没有找到 Rust，请先安装：https://rustup.rs （或 winget install Rustlang.Rustup）" }
Ok "Node $(node -v) · $(cargo --version)"

$vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
$hasMsvc = $false
if (Test-Path $vswhere) {
  $hasMsvc = [bool](& $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath)
}
if (-not $hasMsvc) {
  Die "没有找到 C++ 编译工具。请安装 Visual Studio 生成工具，并勾选「使用 C++ 的桌面开发」：
    winget install Microsoft.VisualStudio.2022.BuildTools --override `"--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended`""
}
Ok "Visual Studio C++ 生成工具"

# ---------------------------------------------------------------- 2. 依赖
Step "安装前端依赖"
$needInstall = -not (Test-Path "node_modules")
if (-not $needInstall -and (Test-Path "node_modules\.package-lock.json")) {
  $needInstall = (Get-Item "package-lock.json").LastWriteTime -gt (Get-Item "node_modules\.package-lock.json").LastWriteTime
}
if ($needInstall) {
  npm install
  if ($LASTEXITCODE -ne 0) { Die "npm install 失败" }
  Ok "依赖已安装"
} else {
  Ok "依赖已是最新，跳过"
}

# ---------------------------------------------------------------- 3. 打包
Step "编译并打包（首次大约 5~10 分钟）"
$version = node -p "require('./src-tauri/tauri.conf.json').version"
$bundleDir = "src-tauri\target\release\bundle\nsis"
if (Test-Path $bundleDir) { Remove-Item -Recurse -Force $bundleDir }
npx tauri build --bundles nsis
if ($LASTEXITCODE -ne 0) { Die "打包失败，请把上面的输出发给我" }

$exe = Get-ChildItem "$bundleDir\*-setup.exe" -ErrorAction SilentlyContinue | Sort-Object LastWriteTime -Descending | Select-Object -First 1
if (-not $exe) { Die "打包完成了，但没有找到安装程序，请把上面的输出发给我" }

# ---------------------------------------------------------------- 4. 整理产物
Step "整理安装包"
New-Item -ItemType Directory -Force -Path "release" | Out-Null
$out = "release\Ankiki_${version}_x64-setup.exe"
Copy-Item $exe.FullName $out -Force
$size = "{0:N1} MB" -f ((Get-Item $out).Length / 1MB)
Ok "$out（$size）"

Start-Process explorer.exe "/select,`"$((Resolve-Path $out).Path)`""

Write-Host ""
Write-Host ("完成！用时 {0:N0} 秒" -f ((Get-Date) - $start).TotalSeconds) -ForegroundColor Green
Write-Host "把 $out 发给别人即可，双击安装，不需要管理员权限。"
