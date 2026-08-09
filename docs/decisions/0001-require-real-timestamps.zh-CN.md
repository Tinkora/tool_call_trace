# ADR-0001：要求真实时间戳，并保持首个版本在浏览器本地运行

[English](0001-require-real-timestamps.md)

## 状态

Accepted

## 日期

2026-08-09

## 背景

首个原型通过同一个模型接受 OpenAI run steps、Generic 数组和 Anthropic message
block。Anthropic message block 能标识工具调用，却不提供调用开始和结束时间。原型用 1ms
duration 和 2ms 间隔填补空缺，使瀑布流看似合理，却产生了错误的延迟和状态结论。

原型还包含类似 MCP 的 JSON schema，但没有 server、transport、注册方式或可执行入口。把这些
schema 作为已完成的 Agent tool 发布，会混淆文档与可工作的集成。

## 决策

- 只接受具有真实时间证据的输入：严格 Generic 数组和带时间戳的 OpenAI run steps。
- 将接受的时间戳归一化到 trace 起点，但绝不编造时间。
- 首个产品界面通过 Rust/WASM 和静态 HTML 保持浏览器本地运行。
- 在具备可执行边界、集成测试、生命周期策略和真实用户需求前，不宣传或发布 MCP/Agent
  transport。

## 考虑过的替代方案

### 保留 Anthropic 占位时间

拒绝。诊断工具不能把合成延迟展示为实测事实。

### 不使用时间轴，只展示 Anthropic 调用列表

暂缓。独立的有序调用视图可能有价值，但将它混入瀑布流模型会削弱当前契约，并在没有需求证据时
增加 UI 复杂度。

### 在浏览器工具之前增加 backend 或 MCP server

首个版本拒绝。它会在本地工作流尚无外部采用证据时扩大信任边界、运维、认证和维护成本。

## 影响

- 不支持的 Anthropic block 会失败，不再渲染误导图表。
- 公开解析 API 更小、更严格。
- 静态页面可以执行“无外部请求”的隐私契约。
- 新格式必须提供真实时间，或引入明确分离的非时间线模型。
- Agent transport 仍是未来产品决策，不是只有文档声明的功能。
