(function attachTcpThroughputModel(root, factory) {
    const model = factory();
    if (typeof module === "object" && module.exports) {
        module.exports = model;
    }
    root.TcpThroughputModel = model;
})(typeof globalThis !== "undefined" ? globalThis : this, function createTcpThroughputModel() {
    "use strict";

    const MIB = 1024 * 1024;
    const DEFAULT_HTTP_HEADER_BYTES = 600;
    const DEFAULT_INITIAL_CWND_SEGMENTS = 10;
    const DEFAULT_RECEIVE_WINDOW_MIB = 64;

    function positiveNumber(value, fallback) {
        const number = Number(value);
        return Number.isFinite(number) && number > 0 ? number : fallback;
    }

    function nonNegativeNumber(value, fallback) {
        const number = Number(value);
        return Number.isFinite(number) && number >= 0 ? number : fallback;
    }

    function transferConfig(input) {
        const rttMs = positiveNumber(input.rttMs, 1);
        const bandwidthMbps = positiveNumber(input.bandwidthMbps, 1);
        const mssBytes = positiveNumber(input.mssBytes, 1460);
        const payloadMiB = positiveNumber(input.payloadMiB, 8);
        const initialCwndSegments = positiveNumber(
            input.initialCwndSegments,
            DEFAULT_INITIAL_CWND_SEGMENTS
        );
        const receiveWindowMiB = positiveNumber(
            input.receiveWindowMiB,
            DEFAULT_RECEIVE_WINDOW_MIB
        );
        const rttSeconds = rttMs / 1000;
        const linkBps = bandwidthMbps * 1e6;
        const payloadBytes = payloadMiB * MIB;
        const initialCwndBytes = initialCwndSegments * mssBytes;
        const receiveWindowBytes = receiveWindowMiB * MIB;
        const bdpBytes = linkBps * rttSeconds / 8;

        return {
            rttMs,
            oneWayMs: rttMs / 2,
            rttSeconds,
            bandwidthMbps,
            linkBps,
            mssBytes,
            payloadMiB,
            payloadBytes,
            initialCwndSegments,
            initialCwndBytes,
            receiveWindowMiB,
            receiveWindowBytes,
            bdpBytes
        };
    }

    function simulateSlidingWindow(input, wireBytes) {
        const config = transferConfig(input);
        const totalWireBytes = positiveNumber(wireBytes, config.payloadBytes);
        const rounds = [];
        let remainingBytes = totalWireBytes;
        let elapsedBeforeRound = 0;
        let congestionWindowBytes = config.initialCwndBytes;
        let deliverySeconds = 0;

        while (remainingBytes > 0) {
            const sendWindowBytes = Math.min(
                congestionWindowBytes,
                config.receiveWindowBytes
            );
            const flightBytes = Math.min(remainingBytes, sendWindowBytes);
            const serializationSeconds = flightBytes * 8 / config.linkBps;
            const finalRound = flightBytes >= remainingBytes;

            rounds.push({
                round: rounds.length + 1,
                elapsedBeforeRound,
                congestionWindowBytes,
                receiveWindowBytes: config.receiveWindowBytes,
                sendWindowBytes,
                flightBytes,
                serializationSeconds,
                finalRound
            });

            remainingBytes -= flightBytes;
            if (finalRound) {
                deliverySeconds = elapsedBeforeRound
                    + serializationSeconds
                    + config.rttSeconds / 2;
                break;
            }

            elapsedBeforeRound += Math.max(config.rttSeconds, serializationSeconds);
            congestionWindowBytes += flightBytes;
        }

        const effectiveBps = config.payloadBytes * 8 / deliverySeconds;

        return {
            ...config,
            wireBytes: totalWireBytes,
            rounds,
            roundCount: rounds.length,
            finalCwndBytes: congestionWindowBytes,
            finalSendWindowBytes: rounds[rounds.length - 1].sendWindowBytes,
            deliverySeconds,
            effectiveBps,
            utilizationPercent: Math.min(100, effectiveBps / config.linkBps * 100)
        };
    }

    function calculateTcpTransfer(input) {
        const config = transferConfig(input);
        return simulateSlidingWindow(input, config.payloadBytes);
    }

    function calculateHttpPost(input) {
        const config = transferConfig(input);
        const requestHeaderBytes = nonNegativeNumber(
            input.requestHeaderBytes,
            DEFAULT_HTTP_HEADER_BYTES
        );
        const requestBytes = config.payloadBytes + requestHeaderBytes;
        const slidingWindow = simulateSlidingWindow(input, requestBytes);
        const responseWaitSeconds = config.rttSeconds / 2;
        const elapsedSeconds = slidingWindow.deliverySeconds + responseWaitSeconds;
        const effectiveBps = config.payloadBytes * 8 / elapsedSeconds;

        return {
            ...slidingWindow,
            requestHeaderBytes,
            requestBytes,
            responseWaitSeconds,
            elapsedSeconds,
            httpPostBps: effectiveBps
        };
    }

    function buildCurve(calculatePoint, input, startMs, endMs, points) {
        const values = [];
        const count = Math.max(2, Math.floor(points || 120));
        const start = positiveNumber(startMs, 1);
        const end = Math.max(start, positiveNumber(endMs, 300));
        for (let index = 0; index < count; index += 1) {
            const ratio = index / (count - 1);
            const rttMs = start + (end - start) * ratio;
            values.push(calculatePoint({ ...input, rttMs }));
        }
        return values;
    }

    function tcpTransferCurve(input, startMs, endMs, points) {
        return buildCurve(calculateTcpTransfer, input, startMs, endMs, points);
    }

    function httpPostCurve(input, startMs, endMs, points) {
        return buildCurve(calculateHttpPost, input, startMs, endMs, points);
    }

    function formatBitsPerSecond(bps) {
        if (!Number.isFinite(bps)) return "∞";
        if (bps >= 1e9) return `${formatNumber(bps / 1e9)} Gbit/s`;
        if (bps >= 1e6) return `${formatNumber(bps / 1e6)} Mbit/s`;
        if (bps >= 1e3) return `${formatNumber(bps / 1e3)} kbit/s`;
        return `${formatNumber(bps)} bit/s`;
    }

    function formatBytesPerSecond(bps) {
        if (!Number.isFinite(bps)) return "∞";
        const bytes = bps / 8;
        if (bytes >= 1e9) return `${formatNumber(bytes / 1e9)} GB/s`;
        if (bytes >= 1e6) return `${formatNumber(bytes / 1e6)} MB/s`;
        if (bytes >= 1e3) return `${formatNumber(bytes / 1e3)} kB/s`;
        return `${formatNumber(bytes)} B/s`;
    }

    function formatBytes(bytes) {
        if (bytes >= MIB) return `${formatNumber(bytes / MIB)} MiB`;
        if (bytes >= 1024) return `${formatNumber(bytes / 1024)} KiB`;
        return `${formatNumber(bytes)} B`;
    }

    function formatNumber(value) {
        const absolute = Math.abs(value);
        const digits = absolute >= 100 ? 0 : absolute >= 10 ? 1 : 2;
        return new Intl.NumberFormat("zh-CN", {
            maximumFractionDigits: digits,
            minimumFractionDigits: 0
        }).format(value);
    }

    return {
        calculateTcpTransfer,
        simulateSlidingWindow,
        tcpTransferCurve,
        calculateHttpPost,
        httpPostCurve,
        formatBitsPerSecond,
        formatBytesPerSecond,
        formatBytes,
        formatNumber
    };
});
