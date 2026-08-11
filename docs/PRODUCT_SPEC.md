# Tool Call Trace Product Specification

[简体中文](PRODUCT_SPEC.zh-CN.md)

## Problem

Agent developers often receive tool-call logs as JSON but lack a quick,
privacy-preserving way to see concurrency, latency, failures, and repeated
calls. General JSON viewers expose structure but not timing relationships;
hosted observability products can be too heavy or inappropriate for sensitive
one-off traces.

## Target workflow

1. Open the static browser tool or run the local CLI.
2. Select a known format or use structural auto-detection.
3. Optionally enable redaction and add exact field paths.
4. Paste or pipe a timestamped trace and analyze it locally.
5. Compare call position and duration on the waterfall.
6. Inspect redacted input, output, status, and error values.
7. Review aggregate latency, error rate, duplicates, and slow calls.

## Input contracts

Every format has a 5 MiB and 100,000-line input limit. Normalized traces have a
2,000-call limit. Call IDs and names must be non-empty, IDs must be unique,
timestamps must be real, and an end timestamp cannot precede its start.

### Generic JSON

A strict flat array with `id`, `name`, `input`, `start_time_ms`, `end_time_ms`,
and `status`. `output` and `error` are optional. Status accepts `success`,
`error`, `cancelled`, and `pending`, plus `completed`, `failed`, and
`in_progress` aliases. Absolute timestamps are normalized to the earliest
start.

### OpenAI run steps

A `data` list of run-step objects. Tool-call steps must contain function calls,
valid JSON function arguments, `created_at`, and a known status. Terminal steps
must provide the corresponding `completed_at`, `failed_at`, `cancelled_at`, or
`expired_at`. `in_progress` calls remain pending at zero duration. Inline
function output is preserved. Unix-second timestamps are checked before
millisecond conversion.

### OpenAI Agents SDK

An exported span array or `{ "data": [...] }` wrapper. Only spans with
`span_data.type == "function"` become calls. `started_at` is RFC 3339;
`ended_at` is RFC 3339 when present. Missing `ended_at` maps to pending at zero
duration, and a span error maps to error status. JSON strings in function input
and output are decoded when possible and otherwise preserved as strings.

### LangChain

A `Run` object, an array, or a `{ "runs": [...] }` wrapper. Nested
`child_runs` are traversed, and only `run_type == "tool"` becomes a call.
`start_time` and optional `end_time` are RFC 3339. Missing `end_time` maps to
pending; a non-null `error` maps to error status. Structured `inputs` and
`outputs` remain JSON values.

### PydanticAI / Logfire

An OTel span array shaped like `exported_spans_as_dict()`. Only spans with
`gen_ai.operation.name == "execute_tool"` become calls. OTel nanosecond or
RFC 3339 timestamps are accepted and normalized to milliseconds. The importer
uses `gen_ai.tool.*` attributes and accepts legacy `tool_arguments` and
`tool_response` values. OTel error status, `logfire.level_num >= 17`, or an
exception event maps to error. Missing end time or a PydanticAI deferral
attribute maps to pending.

Auto-detection chooses a structurally recognized contract. A recognized but
invalid format fails without falling through to a different parser. Fixture
sources record the upstream repository, pinned commit, source path, license,
and adaptation boundary. They are minimal contract samples, not production
traces.

Anthropic message blocks remain unsupported because they do not provide call
start and end timestamps.

## Redaction contract

Redaction is opt-in. It runs on the normalized log before analysis, display, or
successful CLI output and returns a replacement count. The stable marker is
`[REDACTED]`; trace and tool-call IDs remain unchanged.

Rules run deterministically:

1. Exact configured JSON Pointer paths replace the matched value. Paths are
   relative to a call and must target `/input`, `/output`, or `/error`.
2. Case- and separator-insensitive key matching covers `Authorization`,
   `Proxy-Authorization`, `X-API-Key`, API/access/auth/bearer/refresh/session
   token or key names, client/private/secret key names, `password`, `passwd`,
   and `token`.
3. Remaining string values are scanned for HTTP(S) URLs. User-info, query, and
   fragment components are removed while scheme, host, port, and path remain.
4. Arrays and nested objects are traversed recursively.

Redaction is idempotent and does not claim exhaustive credential or personal
data detection. When redaction is requested but parsing fails, the browser and
CLI suppress parser details that could contain user values. Safe JSON Pointer
validation messages remain available.

## Surfaces

- Browser: format menu, off-by-default redaction, additional path input,
  announced replacement count, statistics, waterfall, tooltip, and details.
- CLI: `tool-call-trace check [--format FORMAT] [--redact]
  [--redact-path POINTER] [FILE|-]` with JSON on stdout and diagnostics on
  stderr.
- Rust: format-specific importers, structural auto-detection, normalized model,
  redaction, and analysis.
- WASM: JSON-compatible parsing, redaction, and analysis functions used by the
  browser.

## Non-goals

- Live instrumentation, collection, hosted ingestion, accounts, or storage.
- A remote API, MCP server, or executable Agent transport.
- Inventing timing for formats that do not supply it.
- Automatic always-on redaction or exhaustive PII classification.
- Replacing an APM or production observability platform.

## Acceptance evidence

- Rust contract tests cover all five formats, timestamp normalization, pending
  and error mapping, duplicate IDs, resource limits, configured paths,
  `X-API-Key`, idempotence, and secret-free redaction outcomes.
- CLI process tests cover stdin and files, explicit and auto formats, exit
  codes, JSON output, replacement counts, and secret-free failures.
- Chromium tests exercise compiled WASM at 375, 768, 1024, and 1440 pixels,
  including reparseable redacted output, details, tooltips, parser failures,
  keyboard focus, reduced motion, console output, external traffic, and
  horizontal overflow.
- CI uses pinned Tinkora reusable workflows for Rust, WASM, Pages,
  documentation, release evidence, and supply-chain checks.
