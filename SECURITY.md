# Security Policy

[简体中文](SECURITY.zh-CN.md)

## Supported versions

Tool Call Trace has no public release yet. Security fixes apply to the current
`main` branch until the first version is published.

## Reporting a vulnerability

Use GitHub private vulnerability reporting from the repository's **Security**
tab. Do not disclose a suspected vulnerability in an Issue, Discussion, pull
request, or public trace fixture. If private vulnerability reporting is not
available, do not publish the report; wait for the maintainer to provide a
verified private channel.

Include the affected commit, impact, reproduction steps, browser or target
environment, and a minimized synthetic trace. Never include real credentials,
customer logs, or personal data.

## Security boundary

- Parsing, analysis, and rendering run locally in the browser through WASM.
- The shipped page has no telemetry, analytics, cookies, backend, or external
  network dependency. Playwright fails if the page sends an external request.
- User-controlled trace values are rendered through `textContent`, not
  executable HTML.
- Generic and OpenAI inputs are structurally validated before rendering.
- Inputs larger than 5 MiB and traces over 2,000 calls are rejected.
- Timestamp conversion and duration aggregation are overflow-safe.
- Generated WASM artifacts are validated before browser tests and publication.

## User responsibility

Tool Call Trace does not redact input, output, errors, URLs, paths, secrets, or
personal data. Treat the visible page, screenshots, and copied values as
sensitive until you sanitize them.

## In scope

- Parser denial of service, validation bypass, integer overflow, or panic.
- DOM injection or code execution from trace content.
- Unexpected network traffic or trace exfiltration.
- Unsafe WASM artifact handling or supply-chain compromise in this repository.

Dependency vulnerabilities should also be reported upstream when appropriate,
but a reproducible impact on Tool Call Trace remains in scope here.
