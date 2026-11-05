import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";
import { waffleryDevHost } from "./dev-host.ts";

// https://vite.dev/config/server-options
// WAFFLERY_DATA_DIR (the library a `wafflery start` host owns) turns on development against a
// real host: `vite` serves this source in front of that host (dev-host.ts), with React Fast
// Refresh so an edited component updates in place. The production build uses neither.
const library = process.env.WAFFLERY_DATA_DIR;
export default defineConfig(({ command }) => ({
  base: "/",
  plugins: command === "serve" && library ? [react(), waffleryDevHost(library)] : [],
  server: {
    host: "127.0.0.1",
    port: 5173,
    strictPort: true,
    cors: false,
    // Build and test output is not source: watching it reloads the page whenever tests run.
    watch: { ignored: ["**/storybook-static/**", "**/prebuilt/**", "**/test-results/**", "**/playwright-report/**", "**/.prebuilt-*/**"] },
  },
  preview: { host: "127.0.0.1", port: 4173, strictPort: true },
  build: { target: "es2022" },
}));
