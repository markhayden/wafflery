import { defineConfig, devices } from "@playwright/test";

// https://playwright.dev/docs/test-webserver
export default defineConfig({
  testDir: "./tests/browser",
  globalSetup: "./tests/browser/binaries.ts",
  forbidOnly: true,
  retries: 0,
  workers: 2,
  timeout: 15_000,
  use: {
    baseURL: "http://127.0.0.1:4173",
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
    colorScheme: "dark",
    reducedMotion: "reduce",
  },
  projects: [
    { name: "chromium", use: { ...devices["Desktop Chrome"] } },
    { name: "webkit", use: { ...devices["Desktop Safari"] } },
  ],
  webServer: [
    {
      command: "bun run build && bun run preview --port 4173",
      url: "http://127.0.0.1:4173",
      reuseExistingServer: false,
      timeout: 30_000,
    },
    {
      command: "bun run build:storybook && bun run preview --outDir storybook-static --port 6006",
      url: "http://127.0.0.1:6006",
      reuseExistingServer: false,
      timeout: 30_000,
    },
  ],
});
