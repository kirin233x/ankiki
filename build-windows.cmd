@echo off
rem 双击运行：调用 build-windows.ps1 打包 Windows 安装程序
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0build-windows.ps1"
pause