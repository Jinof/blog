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
  - ai-cowrite
---

MiniMax H3 在 DGX Spark 上跑起来后，我想在局域网外也能调用它。最后搭了一套 Docker + New API + Tailscale Funnel：模型留在 Spark 上，公网请求先经过 New API 的令牌鉴权，再交给 H3 生成视频。

下面记录这次部署的配置和遇到的问题。版本以 2026 年 9 月的环境为准，主机名、域名和令牌用了示例值。

## 请求怎么到达 Spark

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

客户端提交提示词，Spark 负责推理，生成的视频再通过 API 下载。权重不需要搬到云上。

我把 New API 放在模型前面，主要是为了管理调用令牌。Funnel 只负责把请求送进来，不检查模型的 API Token。直接发布没有鉴权的 H3 端口，就等于允许知道地址的人使用这台机器生成视频。

Funnel 省去了配置路由器端口映射的工作。如果访问者都是自己 tailnet 里的设备，用 Tailscale Serve 就够了；需要公网访问时再用 Funnel。它有带宽限制，这次我只用来远程调用和小范围共享。[Tailscale Funnel 官方说明](https://tailscale.com/docs/features/tailscale-funnel)

## 先跑通本机生成

这台 Spark 是 ARM64 架构，GPU 为 GB10。H3 用的是适配过的镜像：

```text
minimax-h3-dgx-spark:sm121-fp8
```

这个镜像是提前构建好、从另一台机器迁过来的，照着名字去公共仓库拉取是拉不到的。构建和兼容补丁参考 [MiniMax-H3-DGX-Spark 项目](https://github.com/joeynyc/MiniMax-H3-DGX-Spark)，权重需要另外准备，并遵循模型自身的许可证。

在接公网之前，先在 Spark 上验证：

```bash
curl -f http://127.0.0.1:8092/health
curl -f http://127.0.0.1:8092/v1/models
```

我先看 `/health`，等它返回 200，再提交一次生成请求。只看 `docker ps` 很容易判断早了：容器已经运行，里面的模型却可能还在读权重。

我这台机器的 H3 冷启动用了约 9 分钟。启动前先用 `free -h` 看可用内存，别同时拉起另一个大模型；Spark 的统一内存也经不起这样叠加占用。

## 用 Docker Compose 部署 New API

我用了 New API `v1.0.0-rc.33`。这是候选版本，后面调视频插件时需要对照源码，所以没有用会变化的 `latest`。镜像仓库 `calciumion/new-api` 见 [New API 官方项目的部署说明](https://github.com/QuantumNous/new-api/tree/v1.0.0-rc.33)。

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

不过，host 网络下的 New API 监听的是 `:3000`，局域网也可能访问这个端口。Funnel 不会替它收紧监听地址。如果只允许从 Funnel 进入，还得配主机防火墙，或者调整容器网络。

我先在本机完成管理员初始化，设置随机密码、关闭公开注册，再开放 Funnel。数据库保存在 `data` 目录，重建容器时别删掉。`.env`、管理员密码和调用令牌也别提交到 Git。

## 给 H3 接上视频接口

H3 的 vLLM-Omni 服务使用视频任务接口：

```text
POST /v1/videos                 提交视频任务
GET  /v1/videos/{id}            查询状态
GET  /v1/videos/{id}/content    下载结果
```

上游要收 multipart 表单，`extra_params` 里还要带上 `task=t2va`、`duration`、`audio_flow_shift` 等参数。普通聊天渠道处理不了这套请求。

我拿这个版本自带的 Sora 任务插件作基础，改了一个 H3 插件：

- 对外使用模型名 `minimax-h3-local`，映射到上游 `/models/MiniMax-H3/FL2VA`。
- 把调用方的视频参数转换成 H3 表单，显式设置 multipart boundary 和对应的 Content-Type。
- 将上游任务状态映射成平台的排队、处理中、完成或失败状态。
- 通过平台下载视频，调用方不需要知道原始模型地址。

`minimax-h3-local` 是我给本机模型起的别名。下面的调用命令依赖这个定制插件，刚装好的 New API 不能直接照着调用。

渠道配置如下：

- 渠道名称：`xspark MiniMax H3`
- 渠道类型：任务插件
- 插件标识：`minimax-h3`
- 对外模型名：`minimax-h3-local`
- 上游地址：`http://127.0.0.1:8092`
- 上游模型名：`/models/MiniMax-H3/FL2VA`

目前接通的是文生视频，还没做参考图上传。可以先用 768×448、2 秒、20 步，一次提交一个任务。

我最初想用 1 步快速测一下，结果 H3 报了 `sigma schedules need at least 2 entries`。后来在插件里加了最低步数检查，实际通路测试改用 4 步。

### 生成成功，但下载返回 502

视频生成完成后，平台进度已经到了 100%，下载却返回 502：

```text
artifact_request_rejected
```

查到最后，是 New API 的媒体抓取防护拦住了 `127.0.0.1:8092`。我保留了 SSRF 防护，只给 H3 开一个例外：允许私有地址，同时启用 IP 白名单，仅放行 `127.0.0.1/32` 和端口 `8092`，域名解析也走这套 IP 过滤。

这套白名单只适合目前接本机 H3 的用途。以后加别的渠道，外部媒体下载也可能被它拦住，到时需要补充允许范围。

改完后，4 步、2 秒的小样成功下载，MP4 为 435,871 字节。至此，提交到下载才算跑通。画质和并发性能没有在这次测试里评估。

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

Funnel 指向 New API 的 3000 端口，H3 继续留在本机 8092。

加上 `--bg` 后，配置会保留下来，设备或 Tailscale 重启后也能恢复。之前用 `ssh -N -L ...` 时需要一直开着终端，换成 Funnel 后就不用了。[Funnel CLI 文档](https://tailscale.com/docs/reference/tailscale-cli/funnel)

检查当前配置与停止入口：

```bash
tailscale funnel status
sudo tailscale funnel --https=443 off
```

我从公网检查了两个结果：网页返回 200，不带令牌请求模型 API 返回 401。首页能打开，不等于鉴权也配对了。

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

拿到任务 ID 后就把它留好。网络超时时，先查原任务，别急着再提交一次，否则可能让 Spark 重复生成同一个视频。

## 重启后自动拉起

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

重启后，New API 和 Funnel 通常先恢复，H3 还得花几分钟加载。管理页面能打开时，模型未必已经可用，仍要看 H3 的 `/health`。

## 主机改名后，旧 Funnel 地址可能失效

主机改名后又遇到一次故障：重启机器，New API 正常，本机 3000 返回 200，Tailscale 也在线，旧公网地址却打不开了。

节点的 Tailscale DNS 名称已经变了，Funnel 还记着旧名字。把这两项放在一起看，就能发现不一致：

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

客户端代理也添了一次乱。域名被解析成 `198.18.x.x` 的 Fake-IP，访问一直超时；换用公共 DNS 查到的 Funnel 中继地址，并保留原域名做 TLS 校验，请求就返回了 200。遇到类似问题，可以先测 Spark 本机服务，再测公网入口，最后查客户端的 DNS 和代理。

目前这套配置已经够我远程调用 H3，也方便给少量客户端使用。还没做并发压测；如果后面用的人多了，再补排队、配额和监控，数据库备份也要单独安排。
