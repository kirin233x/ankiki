import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// 在真机上调试（npm run ios:dev）时，Tauri 会通过 TAURI_DEV_HOST 告诉我们 Mac 的局域网地址
const host = process.env.TAURI_DEV_HOST;

// https://v2.tauri.app/start/frontend/vite/
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
  build: {
    target: "safari14",
    chunkSizeWarningLimit: 1500,
  },
});
