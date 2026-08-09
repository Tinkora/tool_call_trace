# Tool Call Trace

[简体中文](README.zh-CN.md)

[![Pages](https://github.com/Tinkora/tool_call_trace/actions/workflows/pages.yml/badge.svg)](https://github.com/Tinkora/tool_call_trace/actions/workflows/pages.yml)
[![Supply chain](https://github.com/Tinkora/tool_call_trace/actions/workflows/supply-chain.yml/badge.svg)](https://github.com/Tinkora/tool_call_trace/actions/workflows/supply-chain.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-176b55.svg)](LICENSE)

[Open the browser preview](https://tinkora.github.io/tool_call_trace/)

A browser-local waterfall viewer for timestamped AI agent tool calls. It turns
Generic JSON traces and OpenAI run steps into a relative timeline, exposes each
call's input and output, and surfaces latency, errors, repeated calls, and slow
calls without sending the trace to a server.

> Status: pre-release. A public browser preview is deployed through GitHub
> Pages; no versioned release, package, or Agent transport has been published.

## Current capabilities

- Parse a strict Generic JSON array with explicit timestamps and status.
- Parse timestamped OpenAI run steps with `function` tool calls.
- Normalize absolute timestamps to milliseconds from the trace start.
- Show total, average, maximum, error-rate, frequency, duplicate, and slow-call
  findings.
- Inspect untrusted input and output through text-only DOM rendering.
- Run entirely in the browser through Rust and WebAssembly.
- Reject inputs larger than 5 MiB or traces with more than 2,000 calls.

## Deliberate limits

- Anthropic message blocks are not accepted because they do not contain timing
  data. Tool Call Trace never invents placeholder latency.
- This repository does not contain an MCP server, executable Agent tool, or
  remote API. The browser UI and Rust/WASM APIs are the only current surfaces.
- Trace values are not automatically redacted. Remove secrets and personal data
  before sharing screenshots or copied output.
- The tool analyzes static logs; it is not a live tracer or an APM replacement.

## Run locally

Prerequisites: Rust 1.95.0, the `wasm32-unknown-unknown` target,
`wasm-pack` 0.15.0, and Python 3 or another static file server.

```bash
git clone https://github.com/Tinkora/tool_call_trace.git
cd tool_call_trace
rustup target add wasm32-unknown-unknown
wasm-pack build --target web --out-dir static/pkg crates/tool_call_trace_web -- --locked
python3 -m http.server 4174 --bind 127.0.0.1 --directory crates/tool_call_trace_web
```

Open `http://127.0.0.1:4174/static/`.

## Generic input

Each array item must contain `id`, `name`, `input`, `start_time_ms`,
`end_time_ms`, and `status`. IDs must be unique, names and IDs must be non-empty,
and the end time cannot precede the start time. Optional fields are `output` and
`error`. Supported statuses are `success`, `error`, `cancelled`, and `pending`;
the aliases `completed`, `failed`, and `in_progress` are also accepted.

```json
[
  {
    "id": "call_1",
    "name": "search",
    "input": { "query": "WASM" },
    "output": { "matches": 3 },
    "start_time_ms": 1700000000000,
    "end_time_ms": 1700000000250,
    "status": "success"
  }
]
```

## Development

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

The browser suite runs Chromium at 375, 768, 1024, and 1440 pixel widths and
checks the real WASM boundary, the primary workflow, keyboard dialog behavior,
error announcements, reduced motion, console output, external traffic, and
horizontal overflow.

## Documentation

- [Product specification](docs/PRODUCT_SPEC.md)
- [Maturity evidence](docs/MATURITY.md)
- [Contributing](CONTRIBUTING.md)
- [Security](SECURITY.md)
- [Support](SUPPORT.md)
- [Changelog](CHANGELOG.md)

## License

[MIT](LICENSE) Copyright Tinkora contributors.
