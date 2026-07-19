import { defineConfig, devices } from "@playwright/test";

const baseURL = process.env.LEPTOS_SITE_ADDR
  ? `http://${process.env.LEPTOS_SITE_ADDR}`
  : "http://127.0.0.1:3000";

export default defineConfig({
  testDir: ".",
  fullyParallel: true,
  forbidOnly: !!process.env.CI,
  retries: 1,
  timeout: 45_000,
  use: {
    baseURL,
    trace: "on-first-retry",
  },
  projects: [
    { name: "chromium", use: { ...devices["Desktop Chrome"] } },
    { name: "firefox", use: { ...devices["Desktop Firefox"] } },
    ...(process.env.CI
      ? [{ name: "webkit", use: { ...devices["Desktop Safari"] } }]
      : []),
  ],
});
