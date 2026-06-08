---
title: "Minimax Coding Plan Mcp to Skill"
date: 2026-03-08T23:44:03+08:00
draft: true
---

# 背景

Minimax Coding Plan 提供了 MCP 工具 web_search 和 understand_image，但是 OpenClaw 不支持直接调用 MCP 需要转为 Skill。

首先调研一下 MCP 和 Skill 的原理。

# MCP 原理
MCP 是由 Anthropic 提出的 AI 工具暴露协议，支持 STDIO/SSE(HTTP) 两种模式, 分别适用于本地 MCP 服务器和远程 MCP 服务器。
原理流程如下:
```
初始化链接，协议版本对齐并验证权限: Agent -> STDIO/SSE -> MCP 服务器
读取工具列表: Agent -> STDIO/SSE -> MCP 服务器
调用工具: Agent: 工具名称、工具参数-> STDIO/SSE -> MCP 服务器
```

# Skill 原理


# 等等!!!
似乎 mcportr skill 支持直接调用 mcp 服务器。
