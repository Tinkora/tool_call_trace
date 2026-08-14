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

test("redacts normalized traces across the real WASM boundary", async ({ page }) => {
  await page.goto("/static/");

  const result = await page.evaluate(async () => {
    const wasm = await import("/static/pkg/tool_call_trace_web.js");
    await wasm.default();
    const parsed = wasm.wasm_parse_generic_array(JSON.stringify([
      {
        id: "call_searchable_01",
        name: "fetch",
        input: {
          authorization: "Bearer WASM_SECRET_9x",
          "X-API-Key": "WASM_X_API_KEY_SECRET_9x",
          email: "person@example.test",
        },
        start_time_ms: 0,
        end_time_ms: 10,
        status: "success",
      },
    ]));
    const redacted = wasm.wasm_redact_log(JSON.stringify(parsed), JSON.stringify({
      paths: ["/input/email"],
    }));
    return {
      id: redacted.log.calls[0].id,
      authorization: redacted.log.calls[0].input.authorization,
      xApiKey: redacted.log.calls[0].input["X-API-Key"],
      email: redacted.log.calls[0].input.email,
      count: redacted.redacted_values,
    };
  });

  expect(result).toEqual({
    id: "call_searchable_01",
    authorization: "[REDACTED]",
    xApiKey: "[REDACTED]",
    email: "[REDACTED]",
    count: 3,
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
  await page.getByLabel("Format").selectOption("openai");
  await page.getByRole("button", { name: "Analyze trace" }).click();

  await expect(page.locator("#stat-calls")).toHaveText("3");
  await expect(page.locator("#stat-total-time")).toHaveText("30000");
  await expect(page.locator("#stat-error-rate")).toHaveText("33%");
  await expect(page.locator("#stat-most")).toHaveText("-");

  await page.getByRole("button", { name: /^search, success,/ }).click();
  await expect(page.locator("#detail-body")).toContainText('{"matches":3}');
});

test("keeps the selected Agent SDK sample while WASM initializes", async ({ page }) => {
  let markWasmRequested;
  let releaseWasm;
  const wasmRequested = new Promise((resolve) => {
    markWasmRequested = resolve;
  });
  const wasmRelease = new Promise((resolve) => {
    releaseWasm = resolve;
  });
  await page.route("**/*.wasm", async (route) => {
    markWasmRequested();
    await wasmRelease;
    await route.continue();
  });

  await page.goto("/static/");
  await wasmRequested;
  await page.getByLabel("Format").selectOption("pydantic-ai");
  await expect(page.getByRole("textbox", { name: "Tool-call log (JSON)" })).toHaveValue(
    /"gen_ai\.tool\.name": "add_numbers"/,
  );
  releaseWasm();
  await page.getByRole("button", { name: "Analyze trace" }).click();

  await expect(page.locator("#stat-calls")).toHaveText("1");
  await expect(page.locator("#stat-total-time")).toHaveText("40");
  await expect(page.getByRole("button", { name: /^add_numbers, success,/ })).toBeVisible();
});

test("redacts the editor, details, tooltip, and status before rendering", async ({ page }) => {
  await page.goto("/static/");
  await page.getByLabel("Format").selectOption("generic");
  await page.getByRole("checkbox", { name: "Redact common secrets" }).check();
  await page.getByLabel("Additional redaction paths").fill("/input/customer/email");
  await page.getByRole("textbox", { name: "Tool-call log (JSON)" }).fill(JSON.stringify([
    {
      id: "call_searchable_01",
      name: "fetch",
      input: {
        url: "https://user:URL_UI_SECRET_9x@example.test/mcp?token=QUERY_UI_SECRET_9x#fragment",
        authorization: "Bearer AUTH_UI_SECRET_9x",
        "X-API-Key": "X_API_KEY_UI_SECRET_9x",
        customer: { email: "EMAIL_UI_SECRET_9x" },
      },
      error: "Failed https://user:ERROR_UI_SECRET_9x@example.test/mcp?token=ERR_QUERY_9x#fragment",
      start_time_ms: 0,
      end_time_ms: 10,
      status: "error",
    },
  ]));
  await page.getByRole("button", { name: "Analyze trace" }).click();

  await expect(page.getByRole("status")).toHaveText("Redacted 5 values");
  const editorValue = await page.getByRole("textbox", { name: "Tool-call log (JSON)" }).inputValue();
  for (const secret of [
    "URL_UI_SECRET_9x",
    "QUERY_UI_SECRET_9x",
    "AUTH_UI_SECRET_9x",
    "X_API_KEY_UI_SECRET_9x",
    "EMAIL_UI_SECRET_9x",
    "ERROR_UI_SECRET_9x",
    "ERR_QUERY_9x",
  ]) {
    expect(editorValue).not.toContain(secret);
  }

  const row = page.getByRole("button", { name: /^fetch, error,/ });
  await row.hover();
  await expect(page.getByRole("tooltip")).toContainText("https://example.test/mcp");
  await row.click();
  await expect(page.getByRole("dialog")).toContainText("call_searchable_01");
  await expect(page.getByRole("dialog")).toContainText("[REDACTED]");
  const rendered = await page.locator("body").innerText();
  expect(rendered).not.toMatch(/(?:URL|QUERY|AUTH|EMAIL|ERROR)_UI_SECRET_9x|X_API_KEY_UI_SECRET_9x|ERR_QUERY_9x/);

  await page.getByRole("button", { name: "Close call details" }).click();
  await page.getByRole("button", { name: "Analyze trace" }).click();
  await expect(page.locator("#stat-calls")).toHaveText("1");
  await expect(page.getByRole("status")).toHaveText("Redacted 0 values");
});

test("clears the raw editor before reporting a redaction-path error", async ({ page }) => {
  await page.goto("/static/");
  await page.getByRole("checkbox", { name: "Redact common secrets" }).check();
  await page.getByLabel("Additional redaction paths").fill("input/token");
  await page.getByRole("textbox", { name: "Tool-call log (JSON)" }).fill(JSON.stringify([
    {
      id: "call_1",
      name: "fetch",
      input: { authorization: "Bearer ERROR_PATH_SECRET_9x" },
      start_time_ms: 0,
      end_time_ms: 1,
      status: "success",
    },
  ]));
  await page.getByRole("button", { name: "Analyze trace" }).click();

  await expect(page.getByRole("alert")).toContainText("redaction paths must target");
  await expect(page.getByRole("alert")).not.toContainText("ERROR_PATH_SECRET_9x");
  await expect(page.getByRole("textbox", { name: "Tool-call log (JSON)" })).toHaveValue("");
});

test("does not echo invalid trace values when redaction is enabled", async ({ page }) => {
  await page.goto("/static/");
  await page.getByLabel("Format").selectOption("generic");
  await page.getByRole("checkbox", { name: "Redact common secrets" }).check();
  await page.getByRole("textbox", { name: "Tool-call log (JSON)" }).fill(JSON.stringify([
    {
      id: "call_1",
      name: "search",
      input: {},
      start_time_ms: 0,
      end_time_ms: 1,
      status: "STATUS_ERROR_SECRET_9x",
    },
  ]));
  await page.getByRole("button", { name: "Analyze trace" }).click();

  await expect(page.getByRole("alert")).toContainText("input could not be parsed or redacted");
  await expect(page.getByRole("alert")).not.toContainText("STATUS_ERROR_SECRET_9x");
  await expect(page.getByRole("textbox", { name: "Tool-call log (JSON)" })).toHaveValue("");
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
