#!/bin/sh
set -eu

cd "$(dirname "$0")"

if docker compose version >/dev/null 2>&1; then
    compose() { docker compose "$@"; }
else
    compose() { docker-compose "$@"; }
fi

./server/build.sh
compose up --detach --build

attempt=1
while ! compose exec -T client curl --fail --silent --max-time 1 \
    http://172.29.0.2:8080/healthz >/dev/null 2>&1; do
    if [ "$attempt" -ge 30 ]; then
        echo 'router did not become ready' >&2
        compose logs --no-color router >&2
        exit 1
    fi
    attempt=$((attempt + 1))
    sleep 0.25
done

compose exec -T client make-json-body 2500000 /tmp/payload-20mb.json

printf '%s\n' 'Configured RTT check:'
compose exec -T client ping -c 5 172.29.0.2

printf '\n%s\n' 'Single TCP baseline:'
compose exec -T client iperf3 --client 172.29.0.2 --parallel 1 --time 10

printf '\n%s\n' 'HTTP POST (20 Mb = 2,500,000 bytes; curl speed_upload is bytes/second):'
compose exec -T client curl \
    --http1.1 \
    --silent \
    --show-error \
    --output /tmp/upload-response.json \
    --header 'Expect:' \
    --header 'Content-Type: application/json' \
    --data-binary @/tmp/payload-20mb.json \
    --write-out 'size=%{size_upload} speed_Bps=%{speed_upload} connect=%{time_connect} pretransfer=%{time_pretransfer} ttfb=%{time_starttransfer} total=%{time_total}\n' \
    http://172.29.0.2:8080/upload

printf '\n%s\n' 'Go/Gin JSON load report:'
compose exec -T client cat /tmp/upload-response.json
printf '\n'

printf '\n%s\n' 'Router metrics:'
compose logs --no-color --tail 2 router
