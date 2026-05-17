import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const HOST = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    host: HOST || "localhost",
    port: 1420,
    strictPort: true,
    hmr: HOST
      ? {
          protocol: "ws",
          host: HOST,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**", "**/.venv/**", "**/target/**"],
    },
  },
  build: {
    target: "esnext",
    minify: !process.env.TAURI_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_DEBUG,
  },
});
