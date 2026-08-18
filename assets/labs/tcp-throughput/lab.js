(function initializeTcpLab() {
    "use strict";

    const model = window.TcpThroughputModel;
    const root = document.getElementById("tcp-lab");
    const rtt = document.getElementById("rtt");
    const rttOutput = document.getElementById("rtt-output");
    const chart = document.getElementById("throughput-chart");
    if (!model || !root || !rtt || !rttOutput || !chart) return;

    const fixedInput = {
        bandwidthMbps: 1000,
        mssBytes: 1460,
        payloadMiB: 8,
        initialCwndSegments: 10,
        receiveWindowMiB: 64
    };
    const httpPostInput = {
        ...fixedInput,
        requestHeaderBytes: 600
    };
    let renderQueued = false;

    function update() {
        renderQueued = false;
        const rttMs = Number(rtt.value);
        const tcpResult = model.calculateTcpTransfer({ ...fixedInput, rttMs });
        const httpPostResult = model.calculateHttpPost({ ...httpPostInput, rttMs });
        const progress = (rttMs - Number(rtt.min)) / (Number(rtt.max) - Number(rtt.min)) * 100;

        rttOutput.textContent = `${model.formatNumber(rttMs)} ms`;
        rtt.style.setProperty("--range-progress", `${progress}%`);
        drawChart(tcpResult, httpPostResult);
    }

    function scheduleUpdate() {
        if (renderQueued) return;
        renderQueued = true;
        window.requestAnimationFrame(update);
    }

    function drawChart(currentTcp, currentHttpPost) {
        const width = Math.max(300, Math.round(chart.getBoundingClientRect().width || 800));
        const height = width < 520 ? 360 : 500;
        const margin = width < 520
            ? { top: 20, right: 8, bottom: 52, left: 58 }
            : { top: 24, right: 18, bottom: 58, left: 78 };
        const plotWidth = width - margin.left - margin.right;
        const plotHeight = height - margin.top - margin.bottom;
        const xMin = Number(rtt.min);
        const xMax = Number(rtt.max);
        const tcpCurve = model.tcpTransferCurve(fixedInput, xMin, xMax, width < 520 ? 90 : 160);
        const httpPostCurve = model.httpPostCurve(httpPostInput, xMin, xMax, width < 520 ? 90 : 160);
        const yMax = 1.05e9;
        const x = (value) => margin.left + 6 + (value - xMin) / (xMax - xMin) * (plotWidth - 12);
        const y = (value) => margin.top + 6 + (yMax - Math.min(yMax, Math.max(0, value))) / yMax * (plotHeight - 12);
        const tcpLinePath = tcpCurve.map((value, index) => {
            const command = index === 0 ? "M" : "L";
            return `${command}${x(value.rttMs).toFixed(2)},${y(value.effectiveBps).toFixed(2)}`;
        }).join(" ");
        const httpPostLinePath = httpPostCurve.map((value, index) => {
            const command = index === 0 ? "M" : "L";
            return `${command}${x(value.rttMs).toFixed(2)},${y(value.httpPostBps).toFixed(2)}`;
        }).join(" ");
        const xTicks = width < 520 ? [1, 100, 200, 300] : [1, 50, 100, 150, 200, 250, 300];
        const yTicks = [0, 250e6, 500e6, 750e6, 1e9];
        const currentX = x(currentTcp.rttMs);
        const currentTcpY = y(currentTcp.effectiveBps);
        const currentHttpPostY = y(currentHttpPost.httpPostBps);
        const rightAnchored = currentX > width - 190;
        const labelX = rightAnchored ? currentX - 10 : currentX + 10;
        const labelY = Math.max(margin.top + 42, Math.min(currentTcpY, currentHttpPostY) - 12);
        const compactLegend = width < 520;
        const legendX = compactLegend ? margin.left + 54 : margin.left + 112;
        const postLegendX = legendX + (compactLegend ? 76 : 175);
        const postLegendLabel = compactLegend
            ? "POST · 8M"
            : "HTTP POST · 8 MiB · 已建连";

        chart.setAttribute("viewBox", `0 0 ${width} ${height}`);
        chart.innerHTML = `
            <title>Ping / RTT 与 TCP 滑动窗口、HTTP POST 传输速度</title>
            <desc>当前 Ping 为 ${model.formatNumber(currentTcp.rttMs)} 毫秒。在连接已建立、无丢包、IW10、窗口缩放开启且接收窗口上限 64 MiB 的假设下，ACK 滑动窗口传输 8 MiB 的 TCP 平均速度为 ${model.formatBitsPerSecond(currentTcp.effectiveBps)}；上传 8 MiB HTTP POST 并等待响应的有效速度为 ${model.formatBitsPerSecond(currentHttpPost.httpPostBps)}。</desc>
            <rect class="chart-frame" data-chart-frame x="${margin.left}" y="${margin.top}" width="${plotWidth}" height="${plotHeight}"></rect>
            ${yTicks.map((tick) => `
                <line class="chart-grid" x1="${margin.left}" x2="${width - margin.right}" y1="${y(tick)}" y2="${y(tick)}"></line>
                <text class="chart-axis" x="${margin.left - 10}" y="${y(tick) + 4}" text-anchor="end">${shortRate(tick)}</text>
            `).join("")}
            ${xTicks.map((tick, index) => `
                <line class="chart-grid" x1="${x(tick)}" x2="${x(tick)}" y1="${margin.top}" y2="${height - margin.bottom}"></line>
                <text class="chart-axis" x="${x(tick)}" y="${height - margin.bottom + 23}" text-anchor="${index === 0 ? "start" : index === xTicks.length - 1 ? "end" : "middle"}">${tick}</text>
            `).join("")}
            <text class="chart-axis-title" data-axis="y" x="${margin.left}" y="12">传输速度</text>
            <g class="chart-legend" aria-hidden="true">
                <line class="chart-legend-line chart-legend-line-tcp" x1="${legendX}" x2="${legendX + 16}" y1="9" y2="9"></line>
                <text x="${legendX + 22}" y="12">${compactLegend ? "TCP · 8M" : "TCP 滑窗 · 8 MiB · IW10"}</text>
                <line class="chart-legend-line chart-legend-line-post" x1="${postLegendX}" x2="${postLegendX + 16}" y1="9" y2="9"></line>
                <text x="${postLegendX + 22}" y="12">${postLegendLabel}</text>
            </g>
            <text class="chart-axis-title" data-axis="x" x="${margin.left}" y="${height - 8}">PING / RTT (MS)</text>
            <path class="chart-line chart-line-post" d="${httpPostLinePath}"></path>
            <path class="chart-line chart-line-tcp" d="${tcpLinePath}"></path>
            <line class="chart-guide" x1="${currentX}" x2="${currentX}" y1="${margin.top}" y2="${height - margin.bottom}"></line>
            <circle class="chart-marker chart-marker-post" cx="${currentX}" cy="${currentHttpPostY}" r="5"></circle>
            <circle class="chart-marker chart-marker-tcp" cx="${currentX}" cy="${currentTcpY}" r="5"></circle>
            <text class="chart-point-label" x="${labelX}" y="${labelY}" text-anchor="${rightAnchored ? "end" : "start"}">
                <tspan class="chart-point-label-tcp" x="${labelX}" dy="0">TCP ${model.formatBitsPerSecond(currentTcp.effectiveBps)}</tspan>
                <tspan class="chart-point-label-post" x="${labelX}" dy="15">POST ${model.formatBitsPerSecond(currentHttpPost.httpPostBps)}</tspan>
            </text>
        `;
    }

    function shortRate(bps) {
        if (bps === 0) return "0";
        if (bps >= 1e9) return `${model.formatNumber(bps / 1e9)}G`;
        return `${model.formatNumber(bps / 1e6)}M`;
    }

    rtt.addEventListener("input", scheduleUpdate);
    rtt.addEventListener("change", scheduleUpdate);

    if (typeof ResizeObserver === "function") {
        new ResizeObserver(scheduleUpdate).observe(chart);
    } else {
        window.addEventListener("resize", scheduleUpdate);
    }

    update();
})();
