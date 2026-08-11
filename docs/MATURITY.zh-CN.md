# 成熟度证据

[English](MATURITY.md)

当前阶段：**Pre-release**。

## 已有证据

- 核心解析、导入、脱敏和分析具有面向结果的 Rust 契约测试。
- 编译后的 WASM 边界和浏览器流程已在四种视口宽度的 Chromium 中验证。
- `v0.2.0` 提供可复现的静态浏览器归档，以及 checksum、SBOM、许可证证据和构建
  attestation。
- CLI 能从文件或 stdin 校验相同的五种 trace 契约，并为失败提供稳定的非零退出码。
- OpenAI Agents、LangChain 和 PydanticAI fixture 引用了固定的上游契约来源与 MIT
  许可证；它们明确属于适配样本，而不是生产 trace。
- 显式脱敏具备对常见凭据、URL 组件、配置路径、重复处理和无 secret 失败的对抗测试。
- 已配置严格格式化、Clippy、依赖策略、漏洞检查、文档检查和 workflow 安全检查。
- 已启用 Hosted quality workflow、仓库治理、私密漏洞报告和公开 Pages 工具。

## 进入 Alpha 前必须完成

- 记录至少一次使用真实 trace 的外部或维护者工作流，并确认对应 exporter 契约无需针对
  fixture 特殊修改即可工作。

## 进入 Beta 前必须完成

- 与实现工作之外的用户至少完成一次反馈闭环。
- 通过多个版本证明持续维护能力。
- 在增加其他 exporter、实时采集或更广的 export surface 前，以可测量需求为依据。
