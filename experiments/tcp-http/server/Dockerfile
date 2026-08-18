FROM alpine:3.22

RUN apk add --no-cache ethtool iperf3 iproute2
COPY entrypoint.sh /usr/local/bin/server-entrypoint
COPY .build/lab-server /usr/local/bin/lab-server

ENTRYPOINT ["/usr/local/bin/server-entrypoint"]
