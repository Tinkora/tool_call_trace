# 安全策略

[English](SECURITY.md)

## 支持版本

Tool Call Trace 尚未公开发布。首个版本发布前，安全修复适用于当前 `main` 分支。

## 报告漏洞

请在仓库 **Security** 标签页使用 GitHub private vulnerability reporting。不要在
Issue、Discussion、Pull Request 或公开 trace fixture 中披露疑似漏洞。如果 private
vulnerability reporting 不可用，请不要公开报告，等待维护者提供经过验证的私密渠道。

报告应包含受影响 commit、影响、复现步骤、浏览器或 target 环境，以及最小化的合成 trace。
不得包含真实凭据、客户日志或个人数据。

## 安全边界

- 解析、分析和渲染通过 WASM 在浏览器本地完成。
- 发布页面没有 telemetry、analytics、cookie、backend 或外部网络依赖；页面发送外部请求时
  Playwright 会失败。
- 用户控制的 trace 值通过 `textContent` 渲染，不会作为可执行 HTML。
- Generic 和 OpenAI 输入在渲染前进行结构校验。
- 超过 5 MiB 的输入和超过 2,000 次调用的 trace 会被拒绝。
- 时间戳转换和 duration 聚合不会发生整数溢出。
- 生成的 WASM artifact 在浏览器测试和发布前会经过校验。

## 用户责任

Tool Call Trace 不会脱敏输入、输出、错误、URL、路径、凭据或个人数据。完成清理前，应将
可见页面、截图和复制内容视为敏感信息。

## 范围内问题

- Parser 拒绝服务、校验绕过、整数溢出或 panic。
- trace 内容导致的 DOM 注入或代码执行。
- 非预期网络流量或 trace 外泄。
- 本仓库中的不安全 WASM artifact 处理或供应链问题。

依赖漏洞适合时也应向上游报告；但如果能复现其对 Tool Call Trace 的影响，仍属于本项目范围。
