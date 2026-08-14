# Tool Call Trace

[English](README.md)

<!-- markdownlint-disable MD033 -->
<p align="center">
  <a href="https://ko-fi.com/tinkora" target="_blank" rel="noopener noreferrer">
    <img
      src="https://ko-fi.com/img/githubbutton_sm.svg"
      alt="在 Ko-fi 上支持 Tinkora"
      width="520"
    >
  </a>
</p>
<!-- markdownlint-enable MD033 -->

[打开浏览器工具](https://tinkora.github.io/tool_call_trace/)

[下载 v0.2.0 及验证资产](https://github.com/Tinkora/tool_call_trace/releases/tag/v0.2.0)

Tool Call Trace 是一个在浏览器本地运行的 AI Agent 工具调用瀑布流查看器和契约检查器。
它可以导入 Generic JSON、OpenAI run steps、OpenAI Agents SDK span、LangChain Run
和 PydanticAI/Logfire span，并且不会上传 trace。显式开启脱敏后，常见凭据和指定字段会
在分析或显示前被替换。

> 状态：预发布成熟度。`v0.2.0` 新增 Agent SDK 导入、本地脱敏和命令行契约检查器；
> 当前没有发布 Package 或 Agent transport。

## 当前能力

- 自动检测或显式解析五种带时间戳的 trace 契约。
- 将绝对时间或 exporter 时间戳归一化为相对 trace 起点的毫秒数。
- 在支持键盘操作的瀑布流中展示总耗时、平均耗时、最大耗时、错误率、调用频率、
  重复调用和慢调用。
- 只通过纯文本 DOM 节点呈现不可信的输入和输出。
- 显式脱敏常见 authorization、API key、token、password、secret 和 private key 字段，
  自由文本中的凭据赋值和 authorization header、HTTP(S) URL 的 user-info、query、
  fragment，以及精确 JSON Pointer 路径。
- 保留 trace ID 和 call ID，确保排查过程仍可搜索。
- 通过 `tool-call-trace check` 从文件或 stdin 校验并归一化相同契约。
- 拒绝超过 5 MiB 或 100,000 行的输入，以及超过 2,000 次调用的 trace。

## 支持的输入

| 格式 | 接受的带时间戳导出 |
| --- | --- |
| Generic JSON | 包含毫秒时间戳和状态的严格扁平数组 |
| OpenAI run steps | 包含 function tool call step 的 `data` 列表 |
| OpenAI Agents SDK | 包含 RFC 3339 时间的 `trace.span` function span |
| LangChain | Tool `Run` 对象、数组、`runs` wrapper 和 `child_runs` |
| PydanticAI / Logfire | `exported_spans_as_dict()` 产生的 OTel tool span |

Fixture 的来源和固定上游 revision 记录在
[fixture 来源说明](crates/tool_call_trace_core/tests/fixtures/SOURCES.md)中。

Anthropic message block 不包含开始和结束时间，因此明确不支持。Tool Call Trace 不会
编造延迟。

## 本地脱敏

脱敏默认关闭。在浏览器中启用 **Redact common secrets**，或向 CLI 传递 `--redact`。
附加路径是相对于每个归一化 call 的精确 JSON Pointer，例如
`/input/customer/email` 或 `/output/session/token`。

```bash
cargo run -p tool_call_trace_cli -- \
  check --redact --redact-path /input/customer/email trace.json
```

结果会报告替换值数量，并使用 `[REDACTED]` 标记。脱敏是边界明确的安全辅助功能，
不是完整的凭据或个人数据检测器；分享输出前仍需复核。

## 浏览器快速开始

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

## CLI 快速开始

使用 `-` 或省略路径可读取 stdin。`--format` 支持 `auto`、`generic`、
`openai-run-steps`、`openai-agents`、`langchain` 和 `pydantic-ai`。

```bash
cargo run -p tool_call_trace_cli -- check --format auto trace.json
cargo run -p tool_call_trace_cli -- check --redact - < trace.json
```

成功时，归一化 JSON 写入 stdout；诊断和脱敏计数写入 stderr。无效 trace 契约返回
退出码 `1`，无效命令用法返回退出码 `2`。

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

## 明确限制

- 本工具只分析静态日志，不是实时 tracer，也不替代 APM。
- 仓库不包含托管 API、MCP server、可执行 Agent tool、账号系统、存储服务或遥测。
- Exporter 导入以数据契约为边界，不会安装或插桩上游 SDK。
- 脱敏必须显式开启且属于 best-effort；它会保留 trace ID 和 call ID，也不声称完整检测
  PII。

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
边界、解析和脱敏流程、键盘 dialog、无 secret 错误、reduced motion、console、外部请求
和横向溢出。

## 文档

- [产品规格](docs/PRODUCT_SPEC.zh-CN.md)
- [成熟度证据](docs/MATURITY.zh-CN.md)
- [贡献指南](CONTRIBUTING.zh-CN.md)
- [安全策略](SECURITY.zh-CN.md)
- [支持说明](SUPPORT.zh-CN.md)
- [更新记录](CHANGELOG.md)

## 许可证

[MIT](LICENSE)，Copyright Tinkora contributors。
