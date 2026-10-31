import { defineConfig } from "vite";

// https://vite.dev/config/server-options
export default defineConfig({
  base: "./",
  server: {
    host: "127.0.0.1",
    port: 5173,
    strictPort: true,
    cors: false,
  },
  preview: { host: "127.0.0.1", port: 4173, strictPort: true },
  build: { target: "es2022" },
});
