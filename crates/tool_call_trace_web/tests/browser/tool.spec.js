import { expect, test } from "@playwright/test";

test("returns JSON-compatible values across the real WASM boundary", async ({ page }) => {
  await page.goto("/static/");

  const result = await page.evaluate(async () => {
    const wasm = await import("/static/pkg/tool_call_trace_web.js");
    await wasm.default();
    const parsed = wasm.wasm_parse_generic_array(JSON.stringify([
      {
        id: "call_1",
        name: "search",
        input: { query: "hello", filters: ["docs"] },
        output: { count: 1 },
        start_time_ms: 10,
        end_time_ms: 35,
        status: "success",
      },
    ]));
    const analysis = wasm.wasm_analyze(JSON.stringify(parsed), 10);

    return {
      parseIsMap: parsed instanceof Map,
      callsAreArray: Array.isArray(parsed.calls),
      query: parsed.calls[0].input.query,
      roundTrip: JSON.parse(JSON.stringify(parsed)).calls[0].input,
      analysisIsMap: analysis instanceof Map,
      slowCallsAreArray: Array.isArray(analysis.slow_calls),
    };
  });

  expect(result).toEqual({
    parseIsMap: false,
    callsAreArray: true,
    query: "hello",
    roundTrip: { query: "hello", filters: ["docs"] },
    analysisIsMap: false,
    slowCallsAreArray: true,
  });
});

test("analyzes the default trace and exposes the real findings", async ({ page }) => {
  await page.goto("/static/");
  await page.getByRole("button", { name: "Analyze trace" }).click();

  await expect(page.locator("#stat-calls")).toHaveText("9");
  await expect(page.locator("#stat-total-time")).toHaveText("4650");
  await expect(page.locator("#stat-error-rate")).toHaveText("11%");
  await expect(page.getByText("2 repeated calls", { exact: true })).toBeVisible();
  await expect(
    page.getByRole("button", {
      name: "search, success, starts at 0 milliseconds, duration 120 milliseconds",
    }),
  ).toBeVisible();
});

test("uses OpenAI terminal timestamps and preserves function output", async ({ page }) => {
  await page.goto("/static/");
  await page.getByRole("button", { name: "OpenAI run steps" }).click();
  await page.getByRole("button", { name: "Analyze trace" }).click();

  await expect(page.locator("#stat-calls")).toHaveText("3");
  await expect(page.locator("#stat-total-time")).toHaveText("30000");
  await expect(page.locator("#stat-error-rate")).toHaveText("33%");
  await expect(page.locator("#stat-most")).toHaveText("-");

  await page.getByRole("button", { name: /^search, success,/ }).click();
  await expect(page.locator("#detail-body")).toContainText('{"matches":3}');
});

test("announces invalid input next to the editor", async ({ page }) => {
  await page.goto("/static/");
  await page.getByRole("textbox", { name: "Tool-call log (JSON)" }).fill("not json");
  await page.getByRole("button", { name: "Analyze trace" }).click();

  await expect(page.getByRole("alert")).toContainText("Unable to analyze trace: JSON parse error");
  await expect(page.getByLabel("Tool-call timeline")).toHaveText("No trace analyzed");
});

test("opens call details by keyboard and restores focus", async ({ page }) => {
  await page.goto("/static/");
  const analyze = page.getByRole("button", { name: "Analyze trace" });
  await analyze.click();
  await analyze.focus();
  await page.keyboard.press("Tab");

  const firstCall = page.getByRole("button", {
    name: "search, success, starts at 0 milliseconds, duration 120 milliseconds",
  });
  await expect(firstCall).toBeFocused();
  await page.keyboard.press("Enter");
  await expect(page.getByRole("dialog", { name: "Call details: search" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Close call details" })).toBeFocused();
  await page.keyboard.press("Escape");
  await expect(firstCall).toBeFocused();
});

test("has no external traffic, console problems, or page-level overflow", async ({ page }) => {
  const problems = [];
  const requests = [];
  page.on("console", (message) => {
    if (["error", "warning"].includes(message.type())) {
      problems.push(`${message.type()}: ${message.text()}`);
    }
  });
  page.on("pageerror", (error) => problems.push(`pageerror: ${error.message}`));
  page.on("request", (request) => requests.push(request.url()));

  await page.goto("/static/");
  await page.getByRole("button", { name: "Analyze trace" }).click();
  await page.waitForLoadState("networkidle");

  const pageOrigin = new URL(page.url()).origin;
  expect(requests.every((url) => url.startsWith(pageOrigin))).toBe(true);
  expect(problems).toEqual([]);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= document.documentElement.clientWidth,
    ),
  ).toBe(true);
});

test("honors reduced-motion preferences", async ({ page }) => {
  await page.emulateMedia({ reducedMotion: "reduce" });
  await page.goto("/static/");

  const transitionSeconds = await page.getByRole("button", { name: "Analyze trace" }).evaluate(
    (button) => Number.parseFloat(getComputedStyle(button).transitionDuration),
  );
  expect(transitionSeconds).toBeLessThan(0.001);
});
