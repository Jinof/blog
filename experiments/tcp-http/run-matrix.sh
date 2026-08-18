#!/bin/sh
set -eu

cd "$(dirname "$0")"

if docker compose version >/dev/null 2>&1; then
    compose() { docker compose "$@"; }
else
    compose() { docker-compose "$@"; }
fi

RESULTS_DIR=${RESULTS_DIR:-results}
RUNS=${RUNS:-10}
RTT_VALUES=${RTT_VALUES:-"1 10 28 50 100 200 300"}
PAYLOAD_MIB_VALUES=${PAYLOAD_MIB_VALUES:-"1 8 64"}
RESULT_FILE="$RESULTS_DIR/http-post.csv"

mkdir -p "$RESULTS_DIR"
printf '%s\n' 'rtt_ms,payload_bytes,run,speed_Bps,time_connect,time_pretransfer,time_starttransfer,time_total' > "$RESULT_FILE"

./server/build.sh
compose up --detach --build client server

for rtt_ms in $RTT_VALUES; do
    one_way_ms=$(awk -v rtt="$rtt_ms" 'BEGIN {printf "%.3f", rtt / 2}')
    export ROUTER_ONE_WAY_DELAY_MS=$one_way_ms
    compose up --detach --build --force-recreate router

    attempt=1
    while ! compose exec -T client curl --fail --silent --max-time 2 \
        http://172.29.0.2:8080/healthz >/dev/null 2>&1; do
        if [ "$attempt" -ge 30 ]; then
            echo "router did not become ready for RTT $rtt_ms ms" >&2
            compose logs --no-color router >&2
            exit 1
        fi
        attempt=$((attempt + 1))
        sleep 0.25
    done

    for payload_mib in $PAYLOAD_MIB_VALUES; do
        payload_bytes=$((payload_mib * 1024 * 1024))
        compose exec -T client make-json-body "$payload_bytes" /tmp/payload.json

        run=1
        while [ "$run" -le "$RUNS" ]; do
            compose exec -T client curl \
                --http1.1 \
                --silent \
                --show-error \
                --output /dev/null \
                --header 'Expect:' \
                --header 'Content-Type: application/json' \
                --data-binary @/tmp/payload.json \
                --write-out "$rtt_ms,$payload_bytes,$run,%{speed_upload},%{time_connect},%{time_pretransfer},%{time_starttransfer},%{time_total}\\n" \
                http://172.29.0.2:8080/upload >> "$RESULT_FILE"
            run=$((run + 1))
        done
    done
done

printf 'wrote %s\n' "$RESULT_FILE"
