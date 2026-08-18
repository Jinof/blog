#!/bin/sh
set -eu

cd "$(dirname "$0")"

case "$(uname -m)" in
    arm64|aarch64)
        go_arch=arm64
        ;;
    x86_64|amd64)
        go_arch=amd64
        ;;
    *)
        echo "unsupported host architecture: $(uname -m)" >&2
        exit 1
        ;;
esac

build_cache=${TCP_HTTP_GO_CACHE:-${TMPDIR:-/tmp}/tcp-http-lab-go-cache}
mkdir -p .build "$build_cache"

env \
    CGO_ENABLED=0 \
    GOOS=linux \
    GOARCH="$go_arch" \
    GOCACHE="$build_cache" \
    go build -p 1 -trimpath -ldflags="-s -w" -o .build/lab-server .
