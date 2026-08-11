# Tool Call Trace

[English](README.md)

[打开浏览器预览](https://tinkora.github.io/tool_call_trace/)

[下载 v0.1.0 及验证资产](https://github.com/Tinkora/tool_call_trace/releases/tag/v0.1.0)

Tool Call Trace 是一个在浏览器本地运行的 AI Agent 工具调用瀑布流查看器。它把
Generic JSON trace 和 OpenAI run steps 转换为相对时间线，并展示输入、输出、延迟、
错误、重复调用和慢调用；trace 不会上传到服务器。

> 状态：预发布成熟度。`v0.1.0` 是首个版本化浏览器 Release；当前没有 Package 或
> Agent transport。

## 当前能力

- 解析具有明确时间戳和状态的严格 Generic JSON 数组。
- 解析包含 `function` 工具调用和时间戳的 OpenAI run steps。
- 将绝对时间戳归一化为相对 trace 起点的毫秒数。
- 展示总耗时、平均耗时、最大耗时、错误率、调用频率、重复调用和慢调用。
- 只通过文本 DOM 节点呈现不可信的输入和输出。
- 使用 Rust 和 WebAssembly 完全在浏览器本地运行。
- 拒绝超过 5 MiB 的输入或超过 2,000 次调用的 trace。

## 明确限制

- 不接受缺少时间信息的 Anthropic message block，也不会编造占位延迟。
- 当前仓库没有 MCP server、可执行 Agent tool 或远程 API；现有接口只有浏览器 UI
  和 Rust/WASM API。
- 不会自动脱敏。分享截图或复制输出前，请先移除凭据和个人数据。
- 本工具只分析静态日志，不是实时 tracer，也不替代 APM。

## 本地运行

前置条件：Rust 1.95.0、`wasm32-unknown-unknown` target、`wasm-pack` 0.15.0，
以及 Python 3 或其他静态文件服务器。

```bash
git clone https://github.com/Tinkora/tool_call_trace.git
cd tool_call_trace
rustup target add wasm32-unknown-unknown
wasm-pack build --target web --out-dir static/pkg crates/tool_call_trace_web -- --locked
python3 -m http.server 4174 --bind 127.0.0.1 --directory crates/tool_call_trace_web
```

打开 `http://127.0.0.1:4174/static/`。

## Generic 输入

每一项必须包含 `id`、`name`、`input`、`start_time_ms`、`end_time_ms` 和
`status`。ID 必须唯一，ID 和名称不能为空，结束时间不得早于开始时间。`output`
和 `error` 可选。支持 `success`、`error`、`cancelled` 和 `pending`，也接受
`completed`、`failed` 和 `in_progress` 别名。

```json
[
  {
    "id": "call_1",
    "name": "search",
    "input": { "query": "WASM" },
    "output": { "matches": 3 },
    "start_time_ms": 1700000000000,
    "end_time_ms": 1700000000250,
    "status": "success"
  }
]
```

## 开发

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

浏览器测试会在 375、768、1024 和 1440 像素宽度下运行 Chromium，并验证真实 WASM
边界、主流程、键盘 dialog、错误播报、reduced motion、console、外部请求和横向溢出。

## 文档

- [产品规格](docs/PRODUCT_SPEC.zh-CN.md)
- [成熟度证据](docs/MATURITY.zh-CN.md)
- [贡献指南](CONTRIBUTING.zh-CN.md)
- [安全策略](SECURITY.zh-CN.md)
- [支持说明](SUPPORT.zh-CN.md)
- [更新记录](CHANGELOG.md)

## 许可证

[MIT](LICENSE)，Copyright Tinkora contributors。
