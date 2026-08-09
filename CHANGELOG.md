# Changelog

All notable changes will be documented in this file. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and releases will use
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
