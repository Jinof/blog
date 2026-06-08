---
title: "Why Mac Mini Cannot Ping My Rasperry Pi Hostname While My Macbook Can"
date: 2026-03-28T07:12:42+08:00
draft: true
---

# 问题现状

在同一个局域网下有三台机器, Mac Book, Mac Mini 和 Rasperry Pi. Rasperry Pi 的 hostname 为 oopolobo. 分别在两台 Mac 上执行

```bash
ping oopolobo
```

Macbook 成功 ping 通, 而 Mac Mini 无法 ping 通, 令人非常疑惑.

# 初步探索 - AI 端到端

根据 deepseek 的建议, 可添加 .local 后缀再验证, 可能是 Mac Mini 没有自动添加 .local 后缀导致的.

```bash
ping oopolobo.local
```

验证发现添加 .local 后缀就通了. 但是为什么在 Macbook 上和 Mac Mini 上会有不同的表现的呢?

# 深度探索

探索方向1: traceroot/dig 对比解析流程, 对比本地的网络配置, 看看 DNS 配置是否不同.
探索方向2: 阅读 Bonjur 协议, 找找是否有影响解析时自动带 .local 后缀的配置

## 方向1:




