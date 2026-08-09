# 为 Tool Call Trace 贡献

[English](CONTRIBUTING.md)

Tool Call Trace 当前处于 pre-release。在私密 conduct 举报和管理前置条件通过验证前，
Public Issues 和 Discussions 应保持关闭。本文件规定仓库明确开放外部参与后所采用的流程。

## 提议变更前

- 搜索已有工作，并针对 `main` 复现行为。
- 从所有示例和测试 fixture 中移除凭据、个人数据、专有 trace 和客户数据。
- 功能提案应说明真实工作流、现有替代方案、预期收益、成功指标、维护成本和停止条件。
- 疑似漏洞必须通过 private vulnerability reporting 报告，不能创建公开 Issue。

## 开发环境

- Rust 1.95.0
- `wasm32-unknown-unknown`
- `wasm-pack` 0.15.0
- Node.js 24 或更高版本

运行完整本地门禁：

```bash
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo check -p tool_call_trace_web --target wasm32-unknown-unknown --locked
wasm-pack build --target web --out-dir static/pkg crates/tool_call_trace_web -- --locked

cd crates/tool_call_trace_web
npm ci --ignore-scripts
npm run test:wasm-smoke
```

## Pull Request

- 每个 Pull Request 只包含一个完整结果。
- 说明范围、非目标、安全/隐私影响和准确的验证命令。
- 修复行为前先添加失败的回归测试。
- 用户可见语义变化时同步更新中英文文档。
- 重要变化更新 `CHANGELOG.md`。
- trace fixture 必须是合成数据，不得包含凭据或个人数据。

公开 commit 的 subject 和 body 必须使用英文，并遵循
[Conventional Commits](https://www.conventionalcommits.org/)。

前端变更必须保持浏览器本地边界，并通过四种 Playwright 视口。AI agent 修改前端文件时，
必须遵循 `AGENTS.md` 中的 `ui-ux-pro-max` 门禁。

参与即表示同意遵守 [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md)。
