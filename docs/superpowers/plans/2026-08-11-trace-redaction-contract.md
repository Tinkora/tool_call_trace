# Trace Redaction and Contract Implementation Plan

> Status: Tasks 1-5 are complete. Task 6 release validation is in progress.
>
> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:subagent-driven-development` or `superpowers:executing-plans` to
> implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for
> tracking.

**Goal:** Add evidence-backed SDK trace imports, opt-in local redaction, a
contract-checking CLI, and the same workflow in the static browser tool.

**Architecture:** New import modules normalize supported SDK exports into the
existing `ToolCallLog`. A separate redaction module transforms only normalized
logs and returns a count, so CLI and WASM share one security boundary. The UI
calls the same compiled WASM functions and never implements secret matching in
JavaScript.

**Tech Stack:** Rust 2024, serde/serde_json, `time` RFC 3339 parsing, `url`,
wasm-bindgen, static HTML/JavaScript, Playwright.

---

## Task 1: Public SDK Contract Fixtures

**Files:**

- Create: `crates/tool_call_trace_core/tests/fixtures/*.json`
- Create: `crates/tool_call_trace_core/tests/fixtures/SOURCES.md`
- Create: `crates/tool_call_trace_core/tests/import_contracts.rs`

- [x] Add one minimal timestamped tool-call fixture derived from a pinned
  official OpenAI Agents, LangChain, and PydanticAI test/export contract.
- [x] Add failing tests that call the not-yet-implemented format parsers and
  assert stable IDs, names, timing, input, output, and error status.
- [x] Run `cargo test -p tool_call_trace_core --test import_contracts --locked`
  and confirm the tests fail because the parser APIs do not exist.
- [x] Commit the fixtures and red tests as `test: add pinned Agent SDK trace contracts`.

## Task 2: SDK Importers and Auto-Detection

**Files:**

- Create: `crates/tool_call_trace_core/src/import.rs`
- Modify: `crates/tool_call_trace_core/src/lib.rs`
- Modify: `crates/tool_call_trace_core/src/parse.rs`
- Modify: `crates/tool_call_trace_core/src/wasm.rs`
- Modify: `Cargo.toml`
- Modify: `crates/tool_call_trace_core/Cargo.toml`

- [x] Add recognized-shape probes so malformed recognized inputs fail without
  format fallthrough.
- [x] Parse RFC 3339 timestamps with checked millisecond conversion and flatten
  only real tool/function spans.
- [x] Reuse common normalization, duplicate-ID, ordering, duration, and resource
  validation helpers.
- [x] Add the 100,000-line boundary to every raw input parser.
- [x] Run focused importer tests, then workspace tests and Clippy.
- [x] Commit as `feat: import timestamped Agent SDK traces`.

## Task 3: Core Redaction Contract

**Files:**

- Create: `crates/tool_call_trace_core/src/redact.rs`
- Create: `crates/tool_call_trace_core/tests/redaction_contract.rs`
- Modify: `crates/tool_call_trace_core/src/lib.rs`
- Modify: `crates/tool_call_trace_core/src/wasm.rs`
- Modify: `crates/tool_call_trace_core/Cargo.toml`

- [x] Add failing tests for authorization fields, API-key-like keys, URL
  user-info/query/fragment removal, configured JSON Pointer paths, arrays,
  errors, and unchanged call/trace IDs.
- [x] Add failing tests proving secret sentinels are absent from serialized
  outcomes and error messages.
- [x] Implement the smallest recursive transformer with exact configured paths,
  a documented key allowlist, HTTP(S)-only URL parsing, and `[REDACTED]`.
- [x] Expose JSON-compatible WASM redaction and run focused, workspace, Clippy,
  and WASM checks.
- [x] Commit as `feat: add opt-in local trace redaction`.

## Task 4: Contract-Checking CLI

**Files:**

- Create: `crates/tool_call_trace_cli/Cargo.toml`
- Create: `crates/tool_call_trace_cli/src/main.rs`
- Create: `crates/tool_call_trace_cli/tests/cli.rs`
- Modify: `Cargo.toml`

- [x] Add failing process tests for stdin/file input, explicit/auto formats,
  redacted JSON, configured paths, invalid input, and non-zero failures.
- [x] Implement argument parsing without a command framework dependency and
  keep normalized JSON on stdout and diagnostics on stderr.
- [x] Run CLI tests, workspace tests, Clippy, and a manual stdin smoke check.
- [x] Commit as `feat: add the local trace contract CLI`.

## Task 5: Browser Workflow

**Files:**

- Modify: `crates/tool_call_trace_web/src/lib.rs`
- Modify: `crates/tool_call_trace_web/static/index.html`
- Modify: `crates/tool_call_trace_web/tests/browser/tool.spec.js`

- [x] Add failing browser tests for auto-detection, the off-by-default checkbox,
  path validation, announced redaction count, secret-free details/tooltips, and
  preserved IDs.
- [x] Replace the two-option selector with a labeled format menu, add the
  redaction controls, and route all displayed/analyzed data through Rust/WASM.
- [x] Run the compiled WASM smoke suite and Chromium checks at 375, 768, 1024,
  and 1440 pixels, including console, traffic, keyboard, reduced motion, and
  overflow assertions.
- [x] Commit as `feat: expose local trace redaction in the browser`.

## Task 6: Public Documentation and Release Gate

**Files:**

- Modify: `README.md`
- Modify: `README.zh-CN.md`
- Modify: `docs/PRODUCT_SPEC.md`
- Modify: `docs/PRODUCT_SPEC.zh-CN.md`
- Modify: `docs/MATURITY.md`
- Modify: `docs/MATURITY.zh-CN.md`
- Modify: `AGENTS.md`
- Modify: `CHANGELOG.md`

- [x] Document exact formats, limits, redaction rules, CLI usage, limitations,
  and fixture provenance in English-first bilingual docs.
- [x] Remove the obsolete non-goal that says redaction is unavailable.
- [x] Run repository documentation checks and the complete Rust/WASM/browser
  gate from `AGENTS.md`.
- [x] Review staged changes for secret sentinels and unintentional generated
  output, then commit as `docs: document trace contracts and redaction limits`.
- [ ] Push `main`, verify GitHub Actions, and publish a new SemVer release only
  after all required checks, assets, checksums, SBOM, and attestations succeed.
