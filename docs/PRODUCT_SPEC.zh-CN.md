# Tool Call Trace 产品规格

[English](PRODUCT_SPEC.md)

## 问题

Agent 开发者经常拿到 JSON 格式的工具调用日志，却缺少一个轻量、保护隐私的方式来观察
并发关系、延迟、失败和重复调用。普通 JSON 查看器只能展示结构；托管式可观测平台对于
一次性或敏感 trace 又可能过重或不适用。

## 目标工作流

1. 打开静态浏览器工具，或运行本地 CLI。
2. 选择已知格式，或使用结构化自动检测。
3. 按需开启脱敏，并添加精确字段路径。
4. 粘贴或通过管道输入带时间戳的 trace，在本地分析。
5. 在瀑布流中比较调用位置和耗时。
6. 查看脱敏后的输入、输出、状态和错误。
7. 检查聚合延迟、错误率、重复调用和慢调用。

## 输入契约

所有格式的输入上限都是 5 MiB 和 100,000 行；归一化 trace 最多包含 2,000 次调用。
Call ID 和名称不能为空，ID 必须唯一，时间戳必须真实，结束时间不得早于开始时间。

### Generic JSON

严格的扁平数组，包含 `id`、`name`、`input`、`start_time_ms`、
`end_time_ms` 和 `status`；`output` 与 `error` 可选。状态支持 `success`、
`error`、`cancelled` 和 `pending`，也接受 `completed`、`failed` 和
`in_progress` 别名。绝对时间会以最早开始时间为基准归一化。

### OpenAI run steps

包含 run-step 对象的 `data` 列表。工具调用 step 必须包含 function call、合法 JSON
arguments、`created_at` 和已知状态。终态 step 必须提供与状态对应的 `completed_at`、
`failed_at`、`cancelled_at` 或 `expired_at`；`in_progress` 映射为零耗时 pending。
内联 function output 会被保留。Unix 秒级时间戳在转换为毫秒前会检查溢出。

### OpenAI Agents SDK

接受导出 span 数组或 `{ "data": [...] }` wrapper。只有
`span_data.type == "function"` 的 span 会成为 call。`started_at` 使用 RFC 3339；
存在 `ended_at` 时同样使用 RFC 3339。缺少 `ended_at` 会映射为零耗时 pending，span
error 会映射为 error。Function input 和 output 中的 JSON 字符串会尽量解码，无法
解码时保留原字符串。

### LangChain

接受 `Run` 对象、数组或 `{ "runs": [...] }` wrapper，并递归遍历 `child_runs`。
只有 `run_type == "tool"` 会成为 call。`start_time` 和可选 `end_time` 使用
RFC 3339。缺少 `end_time` 映射为 pending；非空 `error` 映射为 error。结构化
`inputs` 和 `outputs` 保持 JSON 值。

### PydanticAI / Logfire

接受与 `exported_spans_as_dict()` 一致的 OTel span 数组。只有包含
`gen_ai.operation.name == "execute_tool"` 的 span 会成为 call。支持 OTel 纳秒或
RFC 3339 时间戳，并归一化为毫秒。Importer 使用 `gen_ai.tool.*` 属性，同时兼容旧的
`tool_arguments` 和 `tool_response`。OTel error status、
`logfire.level_num >= 17` 或 exception event 会映射为 error；缺少结束时间或包含
PydanticAI deferral 属性会映射为 pending。

自动检测会选择结构已识别的契约。已识别但无效的格式会直接失败，不会回退到其他 parser。
Fixture 来源会记录上游仓库、固定 commit、源文件路径、许可证和适配边界。它们是最小契约
样本，不是生产 trace。

Anthropic message block 不提供调用开始和结束时间，因此继续明确不支持。

## 脱敏契约

脱敏必须显式开启。它会在分析、显示或成功的 CLI 输出前处理归一化 log，并返回替换数量。
稳定标记是 `[REDACTED]`；trace ID 和 tool-call ID 保持不变。

规则按确定顺序执行：

1. 精确 JSON Pointer 路径替换匹配值。路径相对于单个 call，并且必须指向
   `/input`、`/output` 或 `/error`。
2. 忽略大小写和分隔符的 key 匹配覆盖 `Authorization`、
   `Proxy-Authorization`、`X-API-Key`、API/access/auth/bearer/refresh/session
   token 或 key 名称、client/private/secret key 名称，以及 `password`、`passwd`
   和 `token`。
3. 其余字符串会扫描 HTTP(S) URL，移除 user-info、query 和 fragment，同时保留
   scheme、host、port 和 path。
4. 数组和嵌套对象会被递归遍历。

脱敏具有幂等性，但不声称完整检测凭据或个人数据。如果已经请求脱敏但解析失败，浏览器和
CLI 会隐藏可能包含用户值的 parser 详情；安全的 JSON Pointer 校验信息仍会显示。

## 对外接口

- 浏览器：format menu、默认关闭的脱敏、附加路径输入、替换数量播报、统计、瀑布流、
  tooltip 和详情。
- CLI：`tool-call-trace check [--format FORMAT] [--redact]
  [--redact-path POINTER] [FILE|-]`，JSON 写入 stdout，诊断写入 stderr。
- Rust：格式专用 importer、结构化自动检测、归一化模型、脱敏和分析。
- WASM：供浏览器使用的 JSON-compatible 解析、脱敏和分析函数。

## 非目标

- 实时插桩、采集、托管 ingestion、账号或存储。
- 远程 API、MCP server 或可执行 Agent transport。
- 为缺少时间信息的格式编造延迟。
- 默认始终开启的自动脱敏或完整 PII 分类。
- 替代 APM 或生产可观测平台。

## 验收证据

- Rust 契约测试覆盖五种格式、时间归一化、pending/error 映射、重复 ID、资源上限、
  配置路径、`X-API-Key`、幂等性和无 secret 的脱敏结果。
- CLI 进程测试覆盖 stdin 和文件、显式与自动格式、退出码、JSON 输出、替换数量和无
  secret 的失败信息。
- Chromium 测试在 375、768、1024 和 1440 像素宽度下验证编译后的 WASM，包括可再次
  解析的脱敏输出、详情、tooltip、parser 失败、键盘焦点、reduced motion、console、
  外部请求和横向溢出。
- CI 使用固定 SHA 的 Tinkora reusable workflow 执行 Rust、WASM、Pages、文档、
  Release 证据和供应链检查。
