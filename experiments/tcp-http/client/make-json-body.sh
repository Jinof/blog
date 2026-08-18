#!/bin/sh
set -eu

total_bytes=${1:?total byte count is required}
output=${2:?output path is required}
json_overhead_bytes=14

if [ "$total_bytes" -le "$json_overhead_bytes" ]; then
    echo "JSON body must be larger than $json_overhead_bytes bytes" >&2
    exit 1
fi

payload_bytes=$((total_bytes - json_overhead_bytes))
{
    printf '{"payload":"'
    head -c "$payload_bytes" /dev/zero | tr '\000' x
    printf '"}'
} > "$output"

actual_bytes=$(wc -c < "$output" | tr -d ' ')
if [ "$actual_bytes" -ne "$total_bytes" ]; then
    echo "generated $actual_bytes bytes, expected $total_bytes" >&2
    exit 1
fi
