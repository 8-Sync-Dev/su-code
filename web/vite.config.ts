import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Base "./" so the built SPA works behind the Rust server at any path.
export default defineConfig({
  base: "./",
  plugins: [react()],
  server: {
    // Dev server proxies API to the Rust backend.
    proxy: {
      "/api": "http://127.0.0.1:8731",
      "/ws": { target: "ws://127.0.0.1:8731", ws: true },
    },
  },
  build: {
    outDir: "dist",
    sourcemap: false,
  },
});
