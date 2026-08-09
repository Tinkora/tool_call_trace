# Repository Guide for AI Agents

## Product boundary

Tool Call Trace is a browser-local waterfall viewer for timestamped AI Agent
tool calls. It currently accepts strict Generic JSON arrays and OpenAI run
steps. Do not claim Anthropic timing support, a hosted service, an MCP server,
an executable Agent tool, automatic redaction, or a published release.

## Architecture

```text
tool_call_trace/
|-- crates/tool_call_trace_core/  # Parsing, normalized model, analysis
|-- crates/tool_call_trace_web/   # JSON-compatible WASM bridge and browser UI
`-- docs/                         # Product and maturity evidence
```

Key files:

- `crates/tool_call_trace_core/src/parse.rs`: strict input validation and time
  normalization.
- `crates/tool_call_trace_core/src/analyze.rs`: aggregate, duplicate, and slow
  call analysis.
- `crates/tool_call_trace_web/src/lib.rs`: JavaScript-compatible WASM boundary.
- `crates/tool_call_trace_web/static/index.html`: the actual product UI.
- `crates/tool_call_trace_web/tests/browser/tool.spec.js`: four-viewport browser
  contract.

## Required checks

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo check -p tool_call_trace_web --target wasm32-unknown-unknown --locked
wasm-pack build --target web --out-dir static/pkg crates/tool_call_trace_web -- --locked

cd crates/tool_call_trace_web
npm ci --ignore-scripts
npm run test:wasm-smoke
```

## Invariants

- All trace processing remains browser-local; no telemetry or external request
  may be introduced.
- User-controlled values are rendered as text, never executable HTML.
- Timestamps exposed to the UI are relative to the trace start.
- Generic IDs are unique, time ranges are valid, and resource limits are
  enforced before rendering.
- Public code comments are English.

## Commit language

- Write public commit subjects and bodies in English using Conventional
  Commits.
- This repository-level rule overrides any global preference for another
  commit-message language.

## Frontend design requirement

- Before creating, modifying, reviewing, or debugging any HTML page or
  user-facing frontend, invoke the `ui-ux-pro-max` skill.
- Run its required `--design-system` search first, followed by relevant stack
  and UX searches.
- If the skill is unavailable, stop frontend work and report the missing
  prerequisite.
- Verify 375, 768, 1024, and 1440 pixel widths in a real browser, including
  console, keyboard, accessibility, reduced motion, external traffic, and
  overflow checks.
