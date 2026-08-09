import { defineConfig } from "@playwright/test";

const testPort = process.env.PLAYWRIGHT_PORT ?? "4174";
if (!/^\d{1,5}$/.test(testPort) || Number(testPort) === 0 || Number(testPort) > 65535) {
  throw new Error("PLAYWRIGHT_PORT must be an integer between 1 and 65535");
}
const testOrigin = `http://127.0.0.1:${testPort}`;

export default defineConfig({
  testDir: "./tests/browser",
  outputDir: "test-results",
  forbidOnly: Boolean(process.env.CI),
  retries: process.env.CI ? 1 : 0,
  reporter: "line",
  use: {
    baseURL: testOrigin,
    screenshot: "only-on-failure",
    trace: "retain-on-failure",
  },
  webServer: {
    command: `npx --no-install http-server . -a 127.0.0.1 -p ${testPort} -c-1 --silent`,
    url: testOrigin,
    reuseExistingServer: false,
  },
  projects: [
    {
      name: "mobile-375",
      use: {
        viewport: { width: 375, height: 812 },
        isMobile: true,
        hasTouch: true,
      },
    },
    {
      name: "tablet-768",
      use: {
        viewport: { width: 768, height: 1024 },
        hasTouch: true,
      },
    },
    {
      name: "desktop-1024",
      use: { viewport: { width: 1024, height: 768 } },
    },
    {
      name: "desktop-1440",
      use: { viewport: { width: 1440, height: 900 } },
    },
  ],
});
