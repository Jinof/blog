FROM alpine:3.22

RUN apk add --no-cache curl ethtool iperf3 iproute2
COPY entrypoint.sh /usr/local/bin/client-entrypoint
COPY make-json-body.sh /usr/local/bin/make-json-body

ENTRYPOINT ["/usr/local/bin/client-entrypoint"]
