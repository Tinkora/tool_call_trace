# Tool Call Trace Product Specification

[简体中文](PRODUCT_SPEC.zh-CN.md)

## Problem

Agent developers often receive tool-call logs as JSON but lack a quick,
privacy-preserving way to see concurrency, latency, failures, and repeated
calls. General JSON viewers expose structure but not timing relationships;
hosted observability products can be too heavy or inappropriate for sensitive
one-off traces.

## Target workflow

1. Open the static tool.
2. Select Generic JSON or OpenAI run steps.
3. Paste a timestamped trace and analyze it locally.
4. Compare call position and duration on the waterfall.
5. inspect a call's input, output, status, and error.
6. Review aggregate latency, error rate, duplicates, and slow calls.

## Supported inputs

### Generic JSON

A strict array of calls with unique IDs, non-empty names, explicit start/end
milliseconds, input JSON, and a supported status. Output and error values are
optional. Absolute timestamps are accepted and normalized to the earliest
start time.

### OpenAI run steps

A `data` array of run-step objects. Tool-call steps must contain function calls,
valid JSON function arguments, `created_at`, and a known status. Terminal steps
must provide the matching `completed_at`, `failed_at`, `cancelled_at`, or
`expired_at` timestamp. An `in_progress` step remains at zero duration until a
terminal timestamp exists. Inline function output is preserved as a string.
Unix-second timestamps are checked before being converted to milliseconds and
normalized.

Anthropic message blocks are intentionally unsupported because their content
does not provide start and end timestamps.

## Current scope

- Browser-local Rust/WASM parsing and analysis.
- Relative horizontal waterfall with status labels and call details.
- Total, average, maximum, error-rate, most-used, duplicate, and slow-call
  findings.
- Input and output rendered through text-only DOM nodes.
- Keyboard-operable rows and native call-details dialog.
- 5 MiB input and 2,000-call resource limits.

## Non-goals

- Live instrumentation, collection, storage, or trace ingestion.
- A hosted API, MCP server, or executable Agent transport.
- Inventing timing for formats that do not supply it.
- Automated secret or personal-data redaction.
- Replacing an APM or production observability platform.

## Acceptance evidence

- Rust tests cover valid parsing, timestamp normalization, invalid structures,
  duplicate IDs, unknown statuses, resource limits, and arithmetic overflow.
- Chromium tests exercise JSON-compatible objects across the compiled WASM
  boundary and cover the main flow, invalid input, keyboard focus, dialog
  recovery, reduced motion, console problems, external requests, and horizontal
  overflow at 375, 768, 1024, and 1440 pixel widths.
- CI uses pinned Tinkora reusable workflows for Rust, WASM, and supply-chain
  checks.
