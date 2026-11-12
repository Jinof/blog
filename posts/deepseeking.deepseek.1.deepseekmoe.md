---
title: "Deepseeking Deepseek - DeepSeek MOE"
date: 2026-09-24T07:02:45+08:00
draft: false
tags:
  - "ai"
  - "notes"
  - "paper reading"
---

[DeepSeekMoE: Towards Ultimate Expert Specialization in Mixture-of-Experts Language Models](https://arxiv.org/abs/2401.06066) 的阅读笔记。

解决的问题：
1. 传统 Top-k MOE 专家知识冗余

解题思路：
1. 增加共享专家承担通用知识
2. 拆分大专家为更小专家，增大知识组合数。

训练的问题：
1. 路由坍缩。部分专家被路由器选择较多，训练较多后可能进一步强化路由器对这些专家的偏好，这些专家进一步训练更多，而其他专家长期得不到充分训练。

解题思路：
1. 专家负载均衡。通过专家负载损失，达到负载均衡。超参数设置的较小防止影响性能。
2. 设备负载均衡。通过设备负载损失，达到负载均衡。
3. 优先设备均衡，其次专家均衡。

论文做的工作：
1. 与 Gshard 2B (Top-K) 对比，训练 DeepSeek MOE 2B，初步验证架构优势。
2. 开展消融实验，通过0共享+16路由、1共享+15路由、1共享+31路由、1共享+63 路由，验证了共享专家和小专家都能提升模型性能。
3. 与 LLama2 7B、DeepSeek 7B(Dense) 对比，训练 DeepSeek MOE 16B，验证了更多的总参数、更少的单次计算可以取得与稠密模型表现相近的结果。
4. 与 DeepSeek 67B 对比，训练 DeepSeek MOE 145B，验证规模放大后，架构优势仍然有效。

个人思考：
1. MOE 还提供了提高吞吐的可能性：因为 Token 计算时必选共享专家，假设共享专家成为瓶颈，冗余部署共享专家。若多个 Token 选择的路由专家不同，则可以同时处理。
