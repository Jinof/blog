#!/bin/sh
set -eu

: "${REMOTE_SUBNET:?REMOTE_SUBNET is required}"
: "${ROUTER_IP:?ROUTER_IP is required}"

interface=$(ip -o route get "$ROUTER_IP" | awk '{for (i = 1; i <= NF; i++) if ($i == "dev") {print $(i + 1); exit}}')
ip route replace "$REMOTE_SUBNET" via "$ROUTER_IP" dev "$interface"
ethtool -K "$interface" rx off tx off tso off gso off gro off lro off >/dev/null 2>&1 || true

exec tail -f /dev/null
