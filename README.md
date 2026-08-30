# Tool Call Trace

[简体中文](README.zh-CN.md)

<!-- markdownlint-disable MD033 -->
<p align="center">
  <a href="https://ko-fi.com/tinkora" target="_blank" rel="noopener noreferrer">
    <img
      src="https://ko-fi.com/img/githubbutton_sm.svg"
      alt="Support Tinkora on Ko-fi"
      width="520"
    >
  </a>
</p>
<!-- markdownlint-enable MD033 -->

[![Pages](https://github.com/Tinkora/tool_call_trace/actions/workflows/pages.yml/badge.svg)](https://github.com/Tinkora/tool_call_trace/actions/workflows/pages.yml)
[![Supply chain](https://github.com/Tinkora/tool_call_trace/actions/workflows/supply-chain.yml/badge.svg)](https://github.com/Tinkora/tool_call_trace/actions/workflows/supply-chain.yml)
[![License: MIT](https://img.shields.io/badge/license-MIT-176b55.svg)](LICENSE)

[Open the browser tool](https://tinkora.github.io/tool_call_trace/)

[Download v0.2.2 and verification assets](https://github.com/Tinkora/tool_call_trace/releases/tag/v0.2.2)

A browser-local waterfall viewer and contract checker for timestamped AI Agent
tool calls. It imports Generic JSON, OpenAI run steps, OpenAI Agents SDK spans,
LangChain Runs, and PydanticAI/Logfire spans without uploading the trace. Opt-in
redaction removes common credentials and selected fields before analysis or
display.

> Status: pre-release maturity. `v0.2.2` adds bounded retry-loop and overlapping
> duplicate findings. No package or Agent transport is published.

## Current capabilities

- Auto-detect or explicitly parse five timestamped trace contracts.
- Normalize absolute or exporter timestamps to milliseconds from trace start.
- Show total, average, maximum, error-rate, frequency, duplicate, slow-call, and
  retry-loop findings in a keyboard-accessible waterfall.
- Inspect untrusted input and output through text-only DOM rendering.
- Explicitly redact common authorization, API-key, token, password, secret,
  and private-key fields; credential assignments and authorization headers in
  free text; HTTP(S) user-info, query, and fragment components; and exact JSON
  Pointer paths.
- Preserve trace and call IDs so investigations remain searchable.
- Validate and normalize the same contracts from files or stdin with
  `tool-call-trace check`.
- Reject input above 5 MiB or 100,000 lines and traces above 2,000 calls.

## Retry-loop findings

The analyzer reports a retry loop only when at least three identical failed
calls run sequentially: each attempt starts after the preceding attempt ends.
An identical success immediately afterward marks the loop as recovered.
Identical calls whose time ranges overlap are reported as one aggregated
overlap group, not as retries. Finding IDs are capped at 20 while `call_count`
retains the complete group size.

Identity uses the trimmed, case-normalized tool name and canonical JSON input.
Analysis happens before optional redaction so replacing distinct credentials
cannot merge unrelated calls. Findings never copy input or error values, but
tool names and call IDs remain user-controlled identifiers and may be sensitive.

## Supported inputs

| Format | Accepted timestamped export |
| --- | --- |
| Generic JSON | Strict flat array with millisecond timestamps and status |
| OpenAI run steps | `data` list of function tool-call run steps |
| OpenAI Agents SDK | Exported `trace.span` function spans with RFC 3339 times |
| LangChain | Tool `Run` objects, arrays, `runs` wrappers, and `child_runs` |
| PydanticAI / Logfire | OTel tool spans from `exported_spans_as_dict()` |

Fixture provenance and pinned upstream revisions are documented in
[fixture sources](crates/tool_call_trace_core/tests/fixtures/SOURCES.md).

Anthropic message blocks are intentionally unsupported because they do not
contain start and end timestamps. Tool Call Trace never invents latency.

## Local redaction

Redaction is off by default. Enable **Redact common secrets** in the browser or
pass `--redact` to the CLI. Additional paths are exact JSON Pointers relative
to each normalized call, such as `/input/customer/email` or
`/output/session/token`.

```bash
cargo run -p tool_call_trace_cli -- \
  check --redact --redact-path /input/customer/email trace.json
```

The result reports the number of replaced values and uses `[REDACTED]` as the
marker. Redaction is a bounded safety aid, not a complete secret or personal
data detector. Review output before sharing it.

## Browser quick start

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

## CLI quick start

Use `-` or omit the path to read stdin. `--format` accepts `auto`, `generic`,
`openai-run-steps`, `openai-agents`, `langchain`, and `pydantic-ai`.

```bash
cargo run -p tool_call_trace_cli -- check --format auto trace.json
cargo run -p tool_call_trace_cli -- check --redact - < trace.json
```

Successful normalized JSON is written to stdout; diagnostics and redaction
counts are written to stderr. The command exits with code `1` for invalid trace
contracts and `2` for invalid command usage.

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

## Deliberate limits

- The tool analyzes static logs; it is not a live tracer or an APM replacement.
- The repository contains no hosted API, MCP server, executable Agent tool,
  account system, storage service, or telemetry.
- Exporter import is contract-based. It does not install or instrument the
  upstream SDKs.
- Redaction is explicit and best-effort. It intentionally preserves trace and
  call IDs and does not claim exhaustive PII detection.

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
checks the real WASM boundary, parsing and redaction workflows, keyboard dialog
behavior, secret-free errors, reduced motion, console output, external traffic,
and horizontal overflow.

## Documentation

- [Product specification](docs/PRODUCT_SPEC.md)
- [Maturity evidence](docs/MATURITY.md)
- [Contributing](CONTRIBUTING.md)
- [Security](SECURITY.md)
- [Support](SUPPORT.md)
- [Changelog](CHANGELOG.md)

## License

[MIT](LICENSE) Copyright Tinkora contributors.
