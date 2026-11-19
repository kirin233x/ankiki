# Android 版（规划中）

Tauri 2 原生支持 Android，和 iOS 一样复用全部界面与核心代码，手机布局（`html.mobile`）已经识别 Android。

## 预计步骤

1. 安装 Android Studio、Android SDK / NDK，设置 `ANDROID_HOME`、`NDK_HOME`
2. `rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android`
3. `npx tauri android init`：生成 `src-tauri/gen/android`
4. `npx tauri android build --apk`：生成安装包

## 需要适配的地方

- **导入词书**：Android 的文件选择器返回的是 `content://` 地址，不能直接当文件路径读取，需要先把文件复制到应用目录再导入
- **图标与启动屏**：素材放在本目录，参考 `platforms/ios/` 用脚本覆盖到生成的工程里
- **返回键**：处理系统返回手势（关闭弹窗、退出学习）
- **CI**：在 `build.yml` 里增加 Android 打包任务，签名密钥放在 GitHub Secrets
