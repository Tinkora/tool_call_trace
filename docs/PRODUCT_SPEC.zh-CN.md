# Tool Call Trace 产品规格

[English](PRODUCT_SPEC.md)

## 问题

Agent 开发者经常拿到 JSON 格式的工具调用日志，却缺少一个轻量、保护隐私的方式来观察
并发关系、延迟、失败和重复调用。普通 JSON 查看器只能展示结构；托管式可观测平台对于
一次性或敏感 trace 又可能过重或不适用。

## 目标工作流

1. 打开静态工具。
2. 选择 Generic JSON 或 OpenAI run steps。
3. 粘贴包含时间戳的 trace，在本地执行分析。
4. 在瀑布流中比较调用位置和耗时。
5. 查看单次调用的输入、输出、状态和错误。
6. 检查聚合延迟、错误率、重复调用和慢调用。

## 支持的输入

### Generic JSON

严格的调用数组：ID 唯一、名称非空、开始和结束毫秒值明确、包含 input JSON，并使用
受支持的状态。output 和 error 可选。允许绝对时间戳，解析后会以最早开始时间为基准归一化。

### OpenAI run steps

包含 run-step 对象的 `data` 数组。工具调用 step 必须包含 function call、合法 JSON
arguments、`created_at` 和已知状态。终态 step 必须提供与状态对应的 `completed_at`、
`failed_at`、`cancelled_at` 或 `expired_at`；`in_progress` 在出现终态时间戳前保持零耗时。
内联 function output 会按字符串保留。Unix 秒级时间戳经过检查后转换为毫秒并归一化。

Anthropic message block 不包含开始和结束时间，因此当前明确不支持。

## 当前范围

- 在浏览器本地使用 Rust/WASM 解析和分析。
- 带状态标签和调用详情的相对水平瀑布流。
- 总耗时、平均耗时、最大耗时、错误率、最常用工具、重复调用和慢调用。
- 通过纯文本 DOM 节点呈现输入和输出。
- 可用键盘操作的时间线行与原生详情 dialog。
- 5 MiB 输入和 2,000 次调用的资源上限。

## 非目标

- 实时插桩、采集、存储或 trace ingestion。
- 托管 API、MCP server 或可执行 Agent transport。
- 为缺少时间信息的格式编造延迟。
- 自动脱敏凭据或个人数据。
- 替代 APM 或生产可观测平台。

## 验收证据

- Rust 测试覆盖正常解析、时间归一化、无效结构、重复 ID、未知状态、资源上限和算术溢出。
- Chromium 测试覆盖编译后 WASM 边界返回 JSON-compatible object 的契约，以及主流程、
  无效输入、键盘焦点、dialog 恢复、reduced motion、console、外部请求和 375、768、
  1024、1440 像素宽度下的横向溢出。
- CI 使用固定 SHA 的 Tinkora reusable workflow 执行 Rust、WASM 和供应链检查。
