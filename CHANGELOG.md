# Changelog

All notable changes will be documented in this file. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and releases will use
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.2] - 2026-08-31

### Added

- Detect sequential retry loops, recovered retries, and aggregated overlapping
  duplicate calls in the browser analysis and CLI JSON report.
- Bound finding call IDs while preserving complete group counts.

### Fixed

- Analyze call identity before optional redaction so distinct secret-bearing
  inputs cannot be merged into a false retry finding.
- Keep the Chinese README release link and status aligned with the current
  published version.
- Add a documentation contract check that rejects stale bilingual release
  links when the workspace package version changes.

## [0.2.1] - 2026-08-15

### Fixed

- Preserve a user-selected sample while the WASM runtime initializes and keep
  analysis disabled until the runtime is ready.

### Security

- Redact credentials inside JSON-encoded output strings and keep parser or
  redaction-boundary failures from echoing user values.
- Redact free-text authorization headers and sensitive `key: value` or
  `key=value` assignments while preserving benign investigation context.

## [0.2.0] - 2026-08-11

### Added

- Timestamped OpenAI Agents SDK, LangChain, and PydanticAI/Logfire importers
  with structural auto-detection and pinned upstream contract provenance.
- A local `tool-call-trace check` CLI for files and stdin, explicit formats,
  normalized JSON output, redaction, and stable failure codes.
- Browser format selection and opt-in redaction with exact JSON Pointer paths,
  replacement counts, and reparseable redacted Generic JSON.

### Security

- Added deterministic redaction for authorization, `X-API-Key`, token,
  password, secret, and private-key-like fields plus sensitive HTTP(S) URL
  components.
- Added a 100,000-line input limit and secret-free CLI and browser failures
  whenever redaction is requested.
- Added adversarial Rust, WASM, CLI, and four-viewport browser coverage for
  redaction boundaries and preserved searchable IDs.

## [0.1.0] - 2026-08-11

### Added

- Strict Generic JSON and timestamped OpenAI run-step parsing.
- Status-specific OpenAI terminal timestamps and preserved inline function
  output.
- Browser-local WASM timeline, call details, statistics, duplicate detection,
  and slow-call findings.
- Real WASM boundary coverage and four-viewport Chromium tests.
- Reusable Rust, WASM, and supply-chain CI workflows.

### Security

- Added a 5 MiB input limit, a 2,000-call limit, unique-ID validation, checked
  timestamp conversion, overflow-safe analysis, text-only DOM rendering, and
  a no-external-request browser gate.

### Removed

- Removed fabricated Anthropic timing support and unimplemented MCP tool
  declarations.

[Unreleased]: https://github.com/Tinkora/tool_call_trace/compare/v0.2.2...HEAD
[0.2.2]: https://github.com/Tinkora/tool_call_trace/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/Tinkora/tool_call_trace/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/Tinkora/tool_call_trace/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/Tinkora/tool_call_trace/releases/tag/v0.1.0
