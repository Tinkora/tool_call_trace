# ADR-0001: Require Real Timestamps and Keep the First Release Browser-Local

[简体中文](0001-require-real-timestamps.zh-CN.md)

## Status

Accepted

## Date

2026-08-09

## Context

The first prototype accepted OpenAI run steps, Generic arrays, and Anthropic
message blocks through one model. Anthropic message blocks identify tool use but
do not provide call start and end timestamps. The prototype filled that gap with
invented 1ms durations and 2ms spacing, which made the waterfall visually
plausible while producing false latency and status conclusions.

The prototype also contained MCP-like JSON schemas without a server, transport,
registration, or executable entry point. Publishing those schemas as completed
Agent tools would confuse documentation with a working integration.

## Decision

- Accept only inputs with real timestamp evidence: strict Generic arrays and
  timestamped OpenAI run steps.
- Normalize accepted timestamps to the trace start, but never invent timing.
- Keep the first product surface browser-local through Rust/WASM and static HTML.
- Do not advertise or ship an MCP/Agent transport until an executable boundary,
  integration tests, lifecycle policy, and user demand exist.

## Alternatives considered

### Keep placeholder Anthropic timing

Rejected because a diagnostic tool must not present synthetic latency as
measured fact.

### Show Anthropic calls without a time axis

Deferred. A separate ordered-call view could be useful, but mixing it into the
waterfall model would weaken the current contract and add UI complexity without
validated demand.

### Add a backend or MCP server before the browser tool

Rejected for the first release. It would expand the trust boundary, operations,
authentication, and maintenance burden before the local workflow has external
adoption evidence.

## Consequences

- Unsupported Anthropic blocks fail instead of rendering a misleading chart.
- Public parsing APIs are smaller and stricter.
- The static page can enforce a no-external-request privacy contract.
- Future formats must provide real timing or introduce an explicitly separate
  non-timeline model.
- An Agent transport remains a future product decision, not a documentation-only
  declaration.
