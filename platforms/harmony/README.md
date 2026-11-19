# 鸿蒙版（规划中）

Tauri 目前不支持鸿蒙 NEXT，所以鸿蒙版需要一个单独的鸿蒙工程（ArkTS），但仍然可以复用大部分代码：

| 部分 | 复用方式 |
| --- | --- |
| 界面 | `npm run build` 生成的 `dist/` 原样放进鸿蒙工程，用 ArkWeb（Web 组件）加载 |
| 核心逻辑 | `src-tauri/core/` 是纯 Rust，可以用 [ohos-rs](https://github.com/ohos-rs/ohos-rs)（类似 napi-rs）编译成鸿蒙原生模块 `.so` |
| 前后端通信 | 把 `src/api.ts` 里的 `invoke` 换成调用鸿蒙原生模块（ArkTS 通过 `javaScriptProxy` 暴露给网页），接口名保持不变 |

## 预计目录结构

```
platforms/harmony/
├── AppScope/               # 应用信息、图标
├── entry/                  # ArkTS 主模块：Web 组件加载界面、桥接调用
├── native/                 # Rust → .so 的绑定层（调用 ankiki-core）
└── build.sh                # 构建界面、编译 .so、hvigor 打包 .hap
```

需要 DevEco Studio 和华为开发者账号（真机调试签名）。
