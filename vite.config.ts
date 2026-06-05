import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(async () => ({
  plugins: [react(), tailwindcss()],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
  server: {
    // Tauri expects the frontend dev server on a fixed port (devUrl=1420 in
    // tauri.conf.json). `tauri dev` runs `npm run dev` WITHOUT setting
    // TAURI_DEV_HOST (that is only set for `tauri dev --host` mobile/network dev),
    // so the port must NOT be keyed on it — doing so made `tauri dev` serve on
    // 1422 while Tauri waited on 1420, so the native app window never launched.
    // Always 1420; the standalone `npm run preview` browser harness overrides to
    // 1422 via its own `--port 1422` flag. strictPort: fail loudly on a port
    // conflict instead of silently serving a port Tauri isn't watching.
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
  },
}));
