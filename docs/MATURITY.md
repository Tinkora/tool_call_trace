# Maturity Evidence

[简体中文](MATURITY.zh-CN.md)

Current stage: **Pre-release**.

## Evidence available

- Core parsing and analysis are covered by outcome-focused Rust tests.
- The compiled WASM boundary is exercised in Chromium.
- The primary browser flow is verified at four viewport widths.
- Version `v0.1.0` is the first reproducible GitHub Release of the static tool.
- Strict formatting, Clippy, dependency policy, and vulnerability checks exist.
- Public claims have been reduced to implemented and reproducible behavior.
- The repository is published under `Tinkora/tool_call_trace` without inherited
  project history.
- Hosted quality workflows, repository governance, private vulnerability
  reporting, and the public Pages preview are verified from the published
  commit.

## Required before Alpha

- Record at least one external or maintainer workflow using a real trace.

## Required before Beta

- Close at least one feedback loop with a user outside the implementation work.
- Demonstrate continued maintenance over multiple releases.
- Decide whether redaction, export, or additional timestamped formats have enough
  measured demand to justify their maintenance cost.
