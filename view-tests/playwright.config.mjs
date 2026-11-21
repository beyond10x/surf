// The live view's page tests: Chromium's headless shell (revision 1254, the one
// @playwright/test 1.65.0 names), one viewport, every test with its own fixture.
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "tests",
  testMatch: "*.spec.mjs",
  outputDir: "../.out/view-test/test-results",
  fullyParallel: true,
  forbidOnly: true,
  retries: 0,
  reporter: "list",
  timeout: 90_000,
  expect: { timeout: 10_000 },
  use: {
    browserName: "chromium",
    headless: true,
    viewport: { width: 1400, height: 900 },
    trace: "off",
    screenshot: "off",
    video: "off",
  },
});
