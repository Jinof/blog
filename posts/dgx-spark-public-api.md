---
title: "dgx spark 算力暴露到公网方式"
date: 2026-09-08T14:45:00+08:00
draft: false
tags:
  - dgx-spark
  - docker
  - tailscale
  - new-api
  - ai
---

我在 DGX Spark 上运行了 MiniMax H3，希望离开局域网后也能提交视频生成任务，并把接口交给其他客户端使用。最后采用的方案是：模型用 Docker 运行，New API 负责账号、令牌和渠道管理，Tailscale Funnel 把管理平台发布为公网 HTTPS 服务。

这篇记录的是 2026 年 9 月的一次实际部署。文中的主机名、域名和令牌均为示例；镜像版本是当时使用的版本，不代表以后部署时的最新选择。

## 整体结构

```text
公网客户端
    │ HTTPS + API Token
    ▼
Tailscale Funnel
    │ 代理到本机 3000
    ▼
New API：账号、令牌、模型渠道、任务记录
    │ H3 视频适配插件
    ▼
127.0.0.1:8092
    │
MiniMax H3 Docker 容器 → GB10 GPU
```

对外提供的是模型 API。模型权重和推理过程仍在 Spark 上，客户端提交提示词，再通过任务接口取回结果。

这里有三个不同的职责：Docker 管进程生命周期，New API 管应用层访问，Funnel 管公网连接。Funnel 不会替模型接口检查 API Token；如果把一个没有鉴权的原始模型端口直接发布出去，知道地址的人就可以调用它。

Funnel 可以为局域网服务提供公网入口，不必自己配置路由器端口映射。如果只想让自己的 Tailscale 设备访问，可以使用 Tailscale Serve；Funnel 面向公网，并有带宽等限制，不应把它当作没有容量上限的商业推理网关。[Tailscale Funnel 官方说明](https://tailscale.com/docs/features/tailscale-funnel)

## 先让模型在本机正常工作

我的机器是 ARM64 的 DGX Spark，GPU 为 GB10。运行 H3 使用了专门适配该机器的镜像：

```text
minimax-h3-dgx-spark:sm121-fp8
```

这是预先构建并迁移过来的本地镜像，不是读者可以直接从公共仓库拉取的通用镜像。模型服务的部署与兼容补丁参考 [MiniMax-H3-DGX-Spark 项目](https://github.com/joeynyc/MiniMax-H3-DGX-Spark)，权重需要另外准备，并遵循模型自身的许可证。

在接公网之前，先在 Spark 上验证：

```bash
curl -f http://127.0.0.1:8092/health
curl -f http://127.0.0.1:8092/v1/models
```

`/health` 返回 200，只表示服务就绪；还应做一次真正的生成请求。`docker ps` 显示容器正在运行时，模型可能还在加载，不能据此认定已经可以推理。

这次 H3 冷启动约需 9 分钟，加载和推理都会占用大量统一内存。启动前要检查 `free -h`，并避免同时启动其他大型模型。这个时间是本次环境的记录，不是所有 Spark 的保证值。

## 用 Docker Compose 部署 New API

这次采用的是 New API `v1.0.0-rc.33`，它是候选版本。固定版本的好处是，排查视频插件行为时可以对应到明确的源码。镜像仓库 `calciumion/new-api` 来自 [New API 官方项目的部署说明](https://github.com/QuantumNous/new-api/tree/v1.0.0-rc.33)。

在 Spark 上准备目录和会话密钥：

```bash
mkdir -p ~/services/new-api/data
cd ~/services/new-api
umask 077
test -f .env || printf 'SESSION_SECRET=%s\n' "$(openssl rand -hex 32)" > .env
```

创建 `compose.yaml`：

```yaml
services:
  new-api:
    image: calciumion/new-api:v1.0.0-rc.33
    container_name: new-api
    restart: unless-stopped
    network_mode: host
    env_file: .env
    environment:
      TZ: Asia/Shanghai
      PORT: "3000"
    volumes:
      - ./data:/data
    mem_limit: 1g
```

然后启动：

```bash
docker compose up -d
curl -f http://127.0.0.1:3000/api/status
```

这里使用 Linux 的 host 网络，是为了让容器内的 New API 能直接访问宿主机的 `127.0.0.1:8092`。普通 bridge 网络下，容器里的 `127.0.0.1` 指向容器自己。

有一个需要说明的边界：这个版本的 New API 在 host 网络下监听 `:3000`，并非只监听 loopback，因此局域网也可能访问 3000 端口。Funnel 代理到 loopback 并不会自动隐藏这个局域网入口；如果要求入口只能经过 Funnel，需要另行配置主机防火墙或更严格的容器网络方案。

在启用 Funnel 之前，先完成管理员初始化、设置随机密码、关闭不需要的公开注册，再创建渠道和调用令牌。`data` 保存数据库等持久化数据，重建容器时应保留它。不要把会话密钥、管理员密码和调用令牌提交到 Git。

## H3 接入不能只填一个聊天渠道

MiniMax H3 是视频生成模型。这次运行的 vLLM-Omni 服务提供了这样的任务接口：

```text
POST /v1/videos                 提交视频任务
GET  /v1/videos/{id}            查询状态
GET  /v1/videos/{id}/content    下载结果
```

H3 上游接收 multipart 表单，还有 `extra_params` 中的 `task=t2va`、`duration`、`audio_flow_shift` 等参数。把地址填进普通聊天渠道，并不能证明这些接口可以正常工作。

我基于该版本 New API 自带的 Sora 任务插件做了一个本地 H3 适配插件，完成以下转换：

- 对外使用模型名 `minimax-h3-local`，映射到上游 `/models/MiniMax-H3/FL2VA`。
- 把调用方的视频参数转换成 H3 表单，显式设置 multipart boundary 和对应的 Content-Type。
- 将上游任务状态映射成平台的排队、处理中、完成或失败状态。
- 通过平台下载视频，调用方不需要知道原始模型地址。

这里的 `minimax-h3-local` 是本次自行配置的别名。New API 并非安装后就自带这个本地 H3 渠道；下文的调用示例以适配插件和渠道已经配置完成为前提。接入其他推理框架时，需要按实际协议调整。

实际配置对应关系如下：

- 渠道名称：`xspark MiniMax H3`
- 渠道类型：任务插件
- 插件标识：`minimax-h3`
- 对外模型名：`minimax-h3-local`
- 上游地址：`http://127.0.0.1:8092`
- 上游模型名：`/models/MiniMax-H3/FL2VA`

目前验证的是文生视频，参考图上传尚未接入。建议从 768×448、2 秒、20 步开始，并一次提交一个任务。这次最初用 1 步做探测，H3 报出 `sigma schedules need at least 2 entries`，因此适配层增加了最低步数检查；不能简单地认为步数越少就越适合测试。

### 生成成功，但下载返回 502

这是另一个容易误判的地方。测试视频已经生成完成，平台也显示了 100%，但下载接口返回：

```text
artifact_request_rejected
```

原因是平台的媒体抓取防护默认不允许访问本机私有地址，而视频恰好来自 `127.0.0.1:8092`。这次保留了 SSRF 防护，改成严格的目标白名单：允许私有地址，同时启用 IP 白名单，仅放行 `127.0.0.1/32` 和端口 `8092`，域名解析也应用 IP 过滤。

这是一台只接本机 H3 的平台所用的限制方式。它会影响其他需要抓取外部媒体的渠道；后续添加新渠道时，应重新设计允许范围，不能直接关闭所有防护来消除 502。

修正后，一次 4 步、2 秒的测试完成了提交、进度查询和 MP4 下载，下载结果为 435,871 字节。这个小样验证的是调用链路，不代表视频质量验收或并发性能测试。

## 用 Tailscale Funnel 发布平台

先按 [Tailscale Linux 安装文档](https://tailscale.com/docs/install/linux) 安装客户端，登录自己的 tailnet：

```bash
sudo tailscale up
```

然后把平台端口发布出去：

```bash
sudo tailscale funnel --bg 3000
```

首次使用可能会返回一个启用链接，需要由有权限的账号完成 Funnel 授权。成功后会显示类似地址：

```text
https://spark.your-tailnet.ts.net/
    → http://127.0.0.1:3000
```

这里发布的是 New API 的 3000 端口，而不是 H3 的 8092 端口。

`--bg` 保存后台配置；重启设备或 Tailscale 后可以自动恢复，不需要保持 SSH 终端一直打开。它与临时执行 `ssh -N -L ...` 的生命周期不同。[Funnel CLI 文档](https://tailscale.com/docs/reference/tailscale-cli/funnel)

检查当前配置与停止入口：

```bash
tailscale funnel status
sudo tailscale funnel --https=443 off
```

公网验证至少分两类：网页应能访问，未带令牌的模型 API 应被拒绝。只检查首页 200，无法验证模型鉴权。

## 一段命令完成生成和下载

先在 New API 控制台创建调用令牌，将下面的域名替换为自己的 Funnel 地址。在装有 `curl` 和 `jq` 的客户端上，用 Bash 执行。令牌从隐藏输入读取，避免直接写进命令历史。

```bash
bash <<'BASH'
set -euo pipefail
BASE='https://spark.your-tailnet.ts.net'
read -r -s -p 'API Token: ' API_KEY </dev/tty
printf '\n'

TASK_ID=$(curl -fsS --max-time 60 "$BASE/v1/videos" \
  -H "Authorization: Bearer $API_KEY" \
  -H 'Content-Type: application/json' \
  -d '{"model":"minimax-h3-local","prompt":"一只橘猫走过阳光下的花园，镜头缓慢跟随，背景有轻柔的鸟鸣。","seconds":2,"size":"768x448","num_inference_steps":20,"seed":42}' \
  | jq -er '.id')

echo "任务：$TASK_ID"
deadline=$((SECONDS + 7200))
while true; do
  RESULT=$(curl -fsS --max-time 30 "$BASE/v1/videos/$TASK_ID" \
    -H "Authorization: Bearer $API_KEY")
  STATUS=$(printf '%s' "$RESULT" | jq -er '.status')
  echo "状态：$STATUS"
  case "$STATUS" in
    completed) break ;;
    failed|cancelled) printf '%s\n' "$RESULT"; exit 1 ;;
    queued|in_progress) ;;
    *) printf '未知状态：%s\n' "$RESULT"; exit 1 ;;
  esac
  if (( SECONDS >= deadline )); then
    echo "等待超时，任务可能仍在运行。请保留任务 ID 稍后查询：$TASK_ID"
    exit 1
  fi
  sleep 10
done

curl -fS --max-time 300 "$BASE/v1/videos/$TASK_ID/content" \
  -H "Authorization: Bearer $API_KEY" \
  -o result.mp4.part
mv result.mp4.part result.mp4
echo '已保存：result.mp4'
BASH
```

视频生成适合这种异步调用：提交后立即得到任务 ID，再轮询和下载。一次网络超时不应立刻重新提交同样的生成任务，应先用原任务 ID 查状态。

## 机器重启后，三层都要恢复

我把 H3 的 Compose 配置设为：

```yaml
restart: always
```

对已经存在的容器，也同步修改运行时策略：

```bash
docker update --restart=always minimax-h3-fl2va
```

Compose 文件和运行容器都要修改，否则下一次重建容器可能又回到旧策略。New API 使用的是 `unless-stopped`：正常重启会恢复，但手动停止的容器会保持停止。`always` 则会在 Docker 守护进程重启后重新启动手动停止的容器。[Docker 重启策略说明](https://docs.docker.com/engine/containers/start-containers-automatically/)

主机服务也要开机启动：

```bash
sudo systemctl enable --now docker tailscaled
systemctl is-enabled docker tailscaled
```

重启后的顺序通常是：Docker 和 Tailscale 启动，New API 与 Funnel 恢复，H3 再完成冷加载。此时管理页面可能已经能打开，但模型还不能生成视频。要分别检查平台首页与 H3 的 `/health`。

## 主机改名后，旧 Funnel 地址可能失效

这次最有迷惑性的故障发生在主机改名并重启以后：New API 容器正常，本机 3000 返回 200，Tailscale 也显示在线，但旧公网地址不能访问。

检查后发现，节点的 Tailscale DNS 名称已经改变，Funnel 配置却仍引用旧名称。诊断时需要同时查看这两项：

```bash
tailscale status --json | jq -r '.Self.DNSName'
tailscale funnel status
```

确认不一致后，重新配置入口。我当时只有这一条 Funnel，使用的是：

```bash
sudo tailscale funnel reset
sudo tailscale funnel --bg 3000
```

`reset` 会清除现有 Serve/Funnel 配置。如果机器承载多个入口，要先记录并有选择地调整，不能直接照抄重置。重建之后，调用脚本中的 `BASE` 和浏览器书签也要使用新的地址。

另外，我的本机代理曾把域名解析成 `198.18.x.x` 的 Fake-IP，普通访问超时，但使用公共 DNS 查询得到的 Funnel 中继地址、保留原域名进行 TLS 校验后，公网访问返回了 200。这说明排查需要把模型、管理平台、Funnel、客户端 DNS/代理分开，不能把所有打不开都归因于容器。

这套方式适合个人设备的远程推理和小范围共享。真正对外稳定提供服务时，还需要按使用规模补上排队、配额、备份和监控。对我这次部署而言，最关键的完成标准是：公网请求必须经过鉴权，并且能够实际提交任务、看到完成状态、拿到视频文件。
