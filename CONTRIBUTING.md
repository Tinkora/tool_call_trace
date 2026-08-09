# Contributing to Tool Call Trace

[简体中文](CONTRIBUTING.zh-CN.md)

Tool Call Trace is currently pre-release. Public Issues and Discussions should
remain disabled until the private conduct-reporting and moderation prerequisites
are verified. This document defines the workflow that applies once the
repository explicitly opens external participation.

## Before proposing a change

- Search existing work and reproduce the behavior against `main`.
- Remove credentials, personal data, proprietary traces, and customer data from
  every example and test fixture.
- For a feature proposal, describe a real workflow, current alternatives,
  expected benefit, success measure, maintenance cost, and stop condition.
- Report suspected vulnerabilities through private vulnerability reporting,
  never through a public Issue.

## Development environment

- Rust 1.95.0
- `wasm32-unknown-unknown`
- `wasm-pack` 0.15.0
- Node.js 24 or newer

Run the complete local gate:

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

## Pull requests

- Keep one coherent outcome per pull request.
- State scope, non-goals, security/privacy effects, and exact verification
  commands.
- Add a failing regression test before fixing behavior.
- Update English and Chinese documentation together when user-visible meaning
  changes.
- Update `CHANGELOG.md` for notable changes.
- Keep all trace fixtures synthetic and free of secrets or personal data.

Public commit subjects and bodies must be in English and follow
[Conventional Commits](https://www.conventionalcommits.org/).

Frontend changes must preserve the browser-local boundary and pass the four
Playwright viewport projects. AI agents working on frontend files must follow
the `ui-ux-pro-max` gate in `AGENTS.md`.

By participating, you agree to follow [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).
