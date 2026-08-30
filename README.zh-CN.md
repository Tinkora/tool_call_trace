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

[下载 v0.2.3 及验证资产](https://github.com/Tinkora/tool_call_trace/releases/tag/v0.2.3)

Tool Call Trace 是一个在浏览器本地运行的 AI Agent 工具调用瀑布流查看器和契约检查器。
它可以导入 Generic JSON、OpenAI run steps、OpenAI Agents SDK span、LangChain Run
和 PydanticAI/Logfire span，并且不会上传 trace。显式开启脱敏后，常见凭据和指定字段会
在分析或显示前被替换。

> 状态：预发布成熟度。`v0.2.3` 新增有界的重试循环和重叠重复调用诊断。
> 当前没有发布 Package 或 Agent transport。

## 当前能力

- 自动检测或显式解析五种带时间戳的 trace 契约。
- 将绝对时间或 exporter 时间戳归一化为相对 trace 起点的毫秒数。
- 在支持键盘操作的瀑布流中展示总耗时、平均耗时、最大耗时、错误率、调用频率、
  重复调用、慢调用和重试循环诊断。
- 只通过纯文本 DOM 节点呈现不可信的输入和输出。
- 显式脱敏常见 authorization、API key、token、password、secret 和 private key 字段，
  自由文本中的凭据赋值和 authorization header、HTTP(S) URL 的 user-info、query、
  fragment，以及精确 JSON Pointer 路径。
- 保留 trace ID 和 call ID，确保排查过程仍可搜索。
- 通过 `tool-call-trace check` 从文件或 stdin 校验并归一化相同契约。
- 拒绝超过 5 MiB 或 100,000 行的输入，以及超过 2,000 次调用的 trace。

## 重试循环诊断

只有至少三次相同失败调用按顺序执行时才报告重试循环：每次尝试必须在上一次结束后
开始。其后紧接的相同成功调用会将循环标为已恢复。时间区间重叠的相同调用会聚合成
一个重叠组，而不会被当成重试。finding 最多保留 20 个调用 ID，`call_count` 仍记录
完整组大小。

调用身份由去除首尾空白并统一大小写的工具名和 canonical JSON 输入决定。分析发生在
可选脱敏之前，避免不同凭据被替换后错误合并。finding 不复制 input 或 error 值，但
工具名和调用 ID 仍是用户控制的标识符，可能包含敏感信息。

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
cargo run -p tool_call_trace_cli -- check --tools tools.json trace.json
```

成功时，归一化 JSON 写入 stdout；诊断和脱敏计数写入 stderr。无效 trace 契约返回
退出码 `1`，无效命令用法返回退出码 `2`。

`--tools FILE` 接受 MCP `tools/list` 结果对象或其中的 `tools` 数组。它会解码
字符串形式的参数、要求参数为 JSON 对象、按工具名精确匹配，并报告 `ARG001` 至
`ARG005`。校验器有意只支持单字符串 `type`、`required`、`properties`、布尔值
`additionalProperties`、`items`、`enum`、`minimum` 和 `maximum`；包含其他校验
关键字的 inventory 会被拒绝。这是有边界的兼容性检查，不是完整 JSON Schema
草案实现。

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
- 参数校验仅离线提供建议；它不会执行、修复调用，也不会推断不同 provider 的工具
  等价性。

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
