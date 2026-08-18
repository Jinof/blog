"use strict";

const assert = require("node:assert/strict");
const model = require("../assets/labs/tcp-throughput/model.js");

const baseInput = {
    bandwidthMbps: 1000,
    mssBytes: 1460,
    payloadMiB: 8,
    initialCwndSegments: 10,
    receiveWindowMiB: 64
};

const cityRtt = model.calculateTcpTransfer({ ...baseInput, rttMs: 28 });
const doubledCityRtt = model.calculateTcpTransfer({ ...baseInput, rttMs: 56 });

assert.equal(cityRtt.initialCwndBytes, 14_600);
assert.equal(cityRtt.rounds[0].sendWindowBytes, 14_600);
assert.equal(cityRtt.rounds[1].sendWindowBytes, 29_200);
assert.equal(cityRtt.roundCount, 10);
assert.equal(cityRtt.rounds.at(-1).finalRound, true);
assert.ok(cityRtt.rounds.every((round) => (
    round.sendWindowBytes === Math.min(
        round.congestionWindowBytes,
        round.receiveWindowBytes
    )
)));
assert.ok(cityRtt.effectiveBps < cityRtt.linkBps);
assert.ok(doubledCityRtt.effectiveBps < cityRtt.effectiveBps);

const receiveLimited = model.calculateTcpTransfer({
    ...baseInput,
    rttMs: 28,
    payloadMiB: 1,
    receiveWindowMiB: 0.05
});
assert.ok(receiveLimited.rounds.every((round) => (
    round.sendWindowBytes <= 0.05 * 1024 * 1024
)));

assert.equal(model.tcpTransferCurve(baseInput, 1, 300, 17).length, 17);
assert.equal(model.formatBitsPerSecond(1_000_000_000), "1 Gbit/s");

const httpPost = model.calculateHttpPost({
    ...baseInput,
    rttMs: 32,
    requestHeaderBytes: 600
});
const slowerHttpPost = model.calculateHttpPost({
    ...baseInput,
    rttMs: 64,
    requestHeaderBytes: 600
});

assert.equal(httpPost.payloadBytes, 8 * 1024 * 1024);
assert.equal(httpPost.requestBytes, 8 * 1024 * 1024 + 600);
assert.equal(httpPost.responseWaitSeconds, 0.016);
assert.equal(httpPost.elapsedSeconds, httpPost.deliverySeconds + 0.016);
assert.ok(httpPost.httpPostBps < httpPost.effectiveBps);
assert.ok(slowerHttpPost.httpPostBps < httpPost.httpPostBps);
assert.equal(model.httpPostCurve(baseInput, 1, 300, 19).length, 19);

console.log("tcp throughput model tests passed");
