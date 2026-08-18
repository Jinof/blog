# TCP / HTTP packet-router experiment

This experiment runs a real end-to-end TCP connection through a Rust IPv4
router. The router forwards Ethernet/IPv4 packets without terminating TCP and
adds an independent FIFO bottleneck in each direction.

## Topology

```text
client 172.28.0.2/24
        |
        |  left interface 172.28.0.254
  Rust packet router
        |  right interface 172.29.0.254
        |
server 172.29.0.2/24
```

The router performs these data-plane operations:

1. Receives an Ethernet/IPv4 frame with an `AF_PACKET` raw socket.
2. Verifies the IPv4 header and destination subnet.
3. Decrements TTL and updates the IPv4 header checksum.
4. Rewrites only the Ethernet next-hop addresses; TCP bytes stay unchanged.
5. Applies seeded random loss and a byte-bounded DropTail queue.
6. Serializes each frame at the configured link rate, waits for propagation
   delay, and transmits it from the opposite interface.

The kernel has `net.ipv4.ip_forward=0`, so there is no second kernel forwarding
path. The client and server disable checksum and segmentation offloads so that
the user-space router sees complete Ethernet frames.

TCP itself is not approximated by a fixed window. The client and server run
real Linux TCP stacks, so congestion window growth, receive-window scaling,
ACK behavior, retransmission, and congestion control all remain end to end.
The Rust process only changes the link conditions seen by that connection.
Inspect the live sliding window during an upload with:

```sh
docker-compose exec -T client ss -tin dst 172.29.0.2:8080
```

## Requirements on macOS

- Docker Desktop, or Colima with the Docker CLI and Compose.
- Go 1.25 or newer to cross-compile the Gin server for the Linux container.
- No host root access is required. Only the router/client/server containers
  receive the narrow `NET_RAW` or `NET_ADMIN` capabilities they need.

The experiment has been validated on Apple silicon with Colima's Docker
runtime. Start it with `colima start` before running the scripts.

A complete Homebrew setup is:

```sh
brew install colima docker docker-compose
colima start --cpu 2 --memory 4 --disk 20
```

The supplied scripts automatically use either `docker compose` or the
standalone `docker-compose` command. They also run `server/build.sh` before
building the small Alpine runtime image, so no Go toolchain image is required.

## Run the default 28 ms RTT experiment

```sh
cd experiments/tcp-http
./run-once.sh
```

Defaults:

- 14 ms propagation delay in each direction (approximately 28 ms RTT)
- 1,000 Mbit/s in each direction
- 0% random loss
- 4 MiB DropTail queue in each direction
- deterministic random seed `1`

The script first measures ping RTT, then runs one `iperf3` TCP stream and sends
a 20 Mb (2,500,000-byte) JSON POST. The Go/Gin server reads the complete body,
uses `encoding/json.Unmarshal` to materialize its 2,499,986-byte `payload`
string, and computes SHA-256 over that string to simulate application CPU and
memory work. The response reports read, unmarshal, hash, and total work time;
`curl` reports `speed_upload` in bytes per second.

Inspect continuously emitted router metrics with:

```sh
docker compose logs --follow router
```

Each JSON line contains receive/forward byte counts, seeded random drops,
DropTail drops, kernel raw-socket drops, route misses, TTL drops, send errors,
current queued bytes, and peak queued bytes for both directions. Each raw
packet socket requests a 16 MiB receive buffer so transient TCP bursts do not
become unreported loss before they reach the Rust queue.

## Configure the virtual link

Compose reads the following environment variables:

```sh
ROUTER_ONE_WAY_DELAY_MS=14
ROUTER_RATE_MBPS=1000
ROUTER_LOSS_PERCENT=0
ROUTER_QUEUE_BYTES=4194304
ROUTER_SEED=1
```

For example:

```sh
ROUTER_ONE_WAY_DELAY_MS=14 \
ROUTER_RATE_MBPS=100 \
ROUTER_LOSS_PERCENT=0.1 \
docker compose up --detach --build --force-recreate router
```

Do not set the link rate to 1.8 MB/s merely to reproduce the observed curl
result. Start at 1 Gbit/s with zero loss, compare `iperf3` with the HTTP sink,
then introduce one impairment at a time.

Stop and remove the experiment containers and networks with:

```sh
docker-compose down
colima stop
```

## Run the RTT and payload matrix

```sh
./run-matrix.sh
```

The default matrix covers RTT values `1, 10, 28, 50, 100, 200, 300 ms`, body
sizes `1, 8, 64 MiB`, and ten new HTTP connections per point. Results are
written to `results/http-post.csv` and are ignored by Git.

Override it without editing the script:

```sh
RTT_VALUES="10 28 50" PAYLOAD_MIB_VALUES="1 8" RUNS=3 ./run-matrix.sh
```

## Verification without Docker

The packet and queue implementation has no crate dependencies:

```sh
cargo test --manifest-path experiments/tcp-http/router/Cargo.toml
cargo check --manifest-path experiments/tcp-http/router/Cargo.toml \
  --target aarch64-unknown-linux-gnu
```

The native macOS binary intentionally exits with an error because `AF_PACKET`
is Linux-only. Docker Desktop or Colima supplies the Linux kernel while keeping
the experiment self-contained on macOS.

## Deliberate limits

- IPv4 only; no IPv6, NAT, ICMP Time Exceeded, fragmentation, or PMTU discovery.
- FIFO DropTail only; no RED, CoDel, FQ-CoDel, ECN, or traffic classes yet.
- Fixed next-hop MACs are resolved at router startup. Recreate the router if a
  client or server container is replaced.
- This models a wired router data plane, not Wi-Fi/LTE radio scheduling.
