import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// Where the web dev server proxies `/api` to (longxia-server). Only used when
// the UI runs in a plain browser; the Tauri webview calls the core via `invoke`
// and never hits `/api`, so this proxy is inert under `tauri dev`.
// @ts-expect-error process is a nodejs global
const apiProxyTarget = process.env.LONGXIA_SERVER || "http://127.0.0.1:8787";

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react()],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**"],
    },
    // 4. web dev only: forward API calls to longxia-server (avoids CORS).
    proxy: {
      "/api": {
        target: apiProxyTarget,
        changeOrigin: true,
      },
    },
  },
}));
