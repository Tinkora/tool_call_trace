# Trace Redaction and Contract Design

## Outcome

Tool Call Trace will accept timestamped exports from OpenAI Agents, LangChain,
and PydanticAI/Logfire in addition to its existing inputs. Users can explicitly
enable local redaction before normalized trace data reaches analysis, display,
JSON output, export, or error rendering. A small CLI will validate the same
contract without uploading the trace.

## Product Boundary

- Processing stays in the Rust core and browser-local WASM runtime.
- Redaction is opt-in and reports how many values changed.
- The default rules cover HTTP(S) URL user-info, query, and fragment values;
  authorization and API-key-like JSON fields; and exact user-configured JSON
  paths.
- Trace and tool-call IDs remain unchanged so investigations stay searchable.
- The project does not claim to detect every secret or personal identifier.
- Hosted ingestion, accounts, storage, vendor exporters, and live tracing stay
  out of scope.

## Input Contracts

All formats share the existing 5 MiB and 2,000-call limits plus a 100,000-line
limit. Every imported tool call must have a non-empty stable ID and name, a
real start time, and an optional end time that does not precede it. Missing end
times map to zero-duration pending calls.

- OpenAI Agents: an array of exported spans. Only `span_data.type ==
  "function"` becomes a tool call. RFC 3339 `started_at` and `ended_at` are
  accepted; `started_at` is required and `ended_at` is optional for pending
  spans. Exporter wrappers use `{ "data": [...] }`.
- LangChain: a run object, an array of runs, or a wrapper with `runs`. Only
  `run_type == "tool"` becomes a tool call. Nested `child_runs` are traversed.
- PydanticAI/Logfire: the public `exported_spans_as_dict()` array. Only spans
  whose attributes contain `gen_ai.operation.name == "execute_tool"` become
  tool calls. OTel nanoseconds and RFC 3339 timestamps are accepted. A missing
  `end_time` or deferral attribute maps to pending.

Auto-detection returns the first structurally recognized format. A recognized
but invalid format fails loudly instead of falling through to another parser.
Fixtures record their upstream repository, commit, source path, and MIT
license so their provenance remains reviewable.

## Redaction Contract

`RedactionConfig` contains exact JSON Pointer paths relative to each normalized
call, such as `/input/customer/email` or `/output/session/token`. The core
returns `RedactionOutcome { log, redacted_values }` and uses the stable marker
`[REDACTED]`.

Traversal is deterministic:

1. Exact configured paths redact the matched value.
2. Object keys matching the documented sensitive-key set, including the
   standard `X-API-Key` header, redact their value.
3. Remaining string values that are valid HTTP(S) URLs lose user-info, query,
   and fragment components while retaining scheme, host, port, and path.
4. Arrays and nested objects are visited recursively with bounded input size
   and call count already enforced by parsing.

The core never includes the raw input document in parse errors, although data
errors can contain an invalid field value. After redaction is requested, the
browser clears its editor before parsing and both browser and CLI suppress
parser details that could echo user values. Successful analysis, display, and
JSON output operate only on the returned redacted log.

## Surfaces

- Rust: format-specific parsers, auto-detection, contract metadata, and
  `redact_log`.
- WASM: auto/explicit parsing and redaction return JSON-compatible objects.
- CLI: `tool-call-trace check [--format ...] [--redact] [--redact-path ...]
  [FILE|-]`; successful output is normalized JSON on stdout and diagnostics go
  to stderr.
- Browser: a format menu, an off-by-default redaction checkbox, optional path
  input, and an announced redaction count. Existing text-only rendering and
  four-viewport behavior remain unchanged.

## Verification

- Contract tests parse one pinned public fixture from each SDK.
- Adversarial tests assert known secret sentinels never occur in normalized
  JSON, CLI output, browser details, tooltips, or errors when redaction is on.
- Tests prove non-secret IDs remain searchable, malformed paths fail loudly,
  and byte/line/call limits apply before rendering.
- Rust formatting, tests, Clippy, WASM build, compiled WASM smoke tests, and
  Chromium at 375, 768, 1024, and 1440 pixels form the release gate.
