#!/bin/sh
set -eu

: "${LEFT_NEXT_HOP_IP:?LEFT_NEXT_HOP_IP is required}"
: "${RIGHT_NEXT_HOP_IP:?RIGHT_NEXT_HOP_IP is required}"
: "${LEFT_SUBNET:?LEFT_SUBNET is required}"
: "${RIGHT_SUBNET:?RIGHT_SUBNET is required}"

route_interface() {
    ip -o route get "$1" | awk '{for (i = 1; i <= NF; i++) if ($i == "dev") {print $(i + 1); exit}}'
}

resolve_mac() {
    target_ip=$1
    target_interface=$2
    attempts=0
    while [ "$attempts" -lt 20 ]; do
        ping -I "$target_interface" -c 1 -W 1 "$target_ip" >/dev/null 2>&1 || true
        target_mac=$(ip neigh show to "$target_ip" dev "$target_interface" | awk '{for (i = 1; i <= NF; i++) if ($i == "lladdr") {print $(i + 1); exit}}')
        if [ -n "${target_mac:-}" ]; then
            printf '%s\n' "$target_mac"
            return 0
        fi
        attempts=$((attempts + 1))
        sleep 0.25
    done
    echo "could not resolve MAC for $target_ip on $target_interface" >&2
    return 1
}

LEFT_IF=$(route_interface "$LEFT_NEXT_HOP_IP")
RIGHT_IF=$(route_interface "$RIGHT_NEXT_HOP_IP")

if [ -z "$LEFT_IF" ] || [ -z "$RIGHT_IF" ] || [ "$LEFT_IF" = "$RIGHT_IF" ]; then
    echo "could not identify two distinct router interfaces" >&2
    exit 1
fi

for interface in "$LEFT_IF" "$RIGHT_IF"; do
    ethtool -K "$interface" rx off tx off tso off gso off gro off lro off >/dev/null 2>&1 || true
done

LEFT_NEXT_HOP_MAC=$(resolve_mac "$LEFT_NEXT_HOP_IP" "$LEFT_IF")
RIGHT_NEXT_HOP_MAC=$(resolve_mac "$RIGHT_NEXT_HOP_IP" "$RIGHT_IF")

exec /usr/local/bin/lab-router \
    --left-if "$LEFT_IF" \
    --right-if "$RIGHT_IF" \
    --left-subnet "$LEFT_SUBNET" \
    --right-subnet "$RIGHT_SUBNET" \
    --left-next-hop-mac "$LEFT_NEXT_HOP_MAC" \
    --right-next-hop-mac "$RIGHT_NEXT_HOP_MAC" \
    --left-to-right-delay-ms "${LEFT_TO_RIGHT_DELAY_MS:-14}" \
    --right-to-left-delay-ms "${RIGHT_TO_LEFT_DELAY_MS:-14}" \
    --left-to-right-rate-mbps "${LEFT_TO_RIGHT_RATE_MBPS:-1000}" \
    --right-to-left-rate-mbps "${RIGHT_TO_LEFT_RATE_MBPS:-1000}" \
    --left-to-right-loss-percent "${LEFT_TO_RIGHT_LOSS_PERCENT:-0}" \
    --right-to-left-loss-percent "${RIGHT_TO_LEFT_LOSS_PERCENT:-0}" \
    --left-to-right-queue-bytes "${LEFT_TO_RIGHT_QUEUE_BYTES:-4194304}" \
    --right-to-left-queue-bytes "${RIGHT_TO_LEFT_QUEUE_BYTES:-4194304}" \
    --seed "${ROUTER_SEED:-1}" \
    --metrics-interval-ms "${METRICS_INTERVAL_MS:-1000}"
