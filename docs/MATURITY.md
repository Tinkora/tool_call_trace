# Maturity Evidence

[简体中文](MATURITY.zh-CN.md)

Current stage: **Pre-release**.

## Evidence available

- Core parsing, import, redaction, and analysis have outcome-focused Rust
  contract tests.
- The compiled WASM boundary and browser workflow are exercised in Chromium at
  four viewport widths.
- `v0.2.1` provides a reproducible static browser archive with checksums, SBOM,
  license evidence, and build attestations.
- The CLI validates the same five trace contracts from files or stdin and uses
  stable non-zero exit codes for failures.
- OpenAI Agents, LangChain, and PydanticAI fixtures cite pinned upstream source
  contracts and MIT licenses. They are explicitly adaptation samples, not
  production traces.
- Opt-in redaction has adversarial coverage for common credentials, URL
  components, configured paths, reprocessing, and secret-free failures.
- Strict formatting, Clippy, dependency policy, vulnerability checks,
  documentation checks, and workflow security checks exist.
- Hosted quality workflows, repository governance, private vulnerability
  reporting, and the public Pages tool are enabled.

## Required before Alpha

- Record at least one external or maintainer workflow using a real trace and
  confirm that its exporter contract works without fixture-specific changes.

## Required before Beta

- Close at least one feedback loop with a user outside the implementation work.
- Demonstrate continued maintenance over multiple releases.
- Use measured demand before adding another exporter, live collection, or a
  broader export surface.
