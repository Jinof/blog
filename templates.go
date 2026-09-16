package main

// Templates retain the existing static HTML and CSS byte for byte.
const style = `
* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}
body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
    line-height: 1.6;
    max-width: 800px;
    margin: 0 auto;
    padding: 2rem 1rem;
    color: #333;
}
header {
    margin-bottom: 3rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid #eee;
}
header h1 a {
    text-decoration: none;
    color: #333;
    font-size: 1.8rem;
}
.post-list {
    list-style: none;
}
.post-item {
    margin-bottom: 1.5rem;
}
.post-item a {
    text-decoration: none;
    color: #0066cc;
    font-size: 1.2rem;
}
.post-item a:hover {
    text-decoration: underline;
}
.post-meta {
    color: #666;
    font-size: 0.9rem;
    margin-top: 0.3rem;
}
.post-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    margin-top: 0.75rem;
}
.post-tags a {
    color: #666;
    font-size: 0.85rem;
    text-decoration: none;
}
.post-tags a:hover {
    color: #0066cc;
}
.post-content {
    margin-top: 2rem;
}
.post-content h1 {
    font-size: 2rem;
    margin-bottom: 1rem;
}
.post-content h2,
.post-content h3,
.post-content h4 {
    margin-top: 1.5rem;
    margin-bottom: 0.75rem;
}
.post-content p,
.post-content ul,
.post-content ol,
.post-content pre,
.post-content blockquote {
    margin-bottom: 1rem;
}
.post-content ul,
.post-content ol {
    padding-left: 1.5rem;
}
.post-content blockquote {
    border-left: 3px solid #ddd;
    color: #555;
    padding-left: 1rem;
}
.post-content code {
    background: #f4f4f4;
    padding: 0.2rem 0.4rem;
    border-radius: 3px;
    font-size: 0.9em;
}
.post-content pre {
    background: #f4f4f4;
    padding: 1rem;
    overflow-x: auto;
    border-radius: 5px;
}
.post-content pre code {
    background: none;
    padding: 0;
}
.post-content img {
    max-width: 100%;
    height: auto;
}
.post-content hr {
    border: 0;
    border-top: 1px solid #eee;
    margin: 1.5rem 0;
}
.back-link {
    display: inline-block;
    margin-bottom: 1.5rem;
    text-decoration: none;
    color: #666;
}
.back-link:hover {
    color: #0066cc;
}
footer {
    margin-top: 3rem;
    padding-top: 1rem;
    border-top: 1px solid #eee;
    color: #666;
    font-size: 0.9rem;
    text-align: center;
}
.empty-state {
    color: #666;
}
.post-item a.lab-entry {
    display: grid;
    gap: 0.3rem;
    margin-top: 0.75rem;
    padding: 1rem 1.15rem;
    border: 1px solid #d8e3df;
    background: #f4faf7;
    color: #183d32;
    text-decoration: none;
}
.post-item a.lab-entry:hover {
    border-color: #1a8a68;
    text-decoration: none;
}
.lab-entry-kicker {
    color: #1a8a68;
    font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
    font-size: 0.72rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
}
.lab-entry strong {
    font-size: 1.15rem;
}
.lab-entry > span:last-child {
    color: #4d655e;
    font-size: 0.9rem;
}
`

const homeStyle = `
* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}
html,
body {
    width: 100%;
    height: 100%;
}
body {
    overflow: hidden;
    background: #fff;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
}
.home-stage {
    position: relative;
    width: 100vw;
    height: 100vh;
    overflow: hidden;
    background: #fff;
}
.home-stage canvas {
    display: block;
    width: 100%;
    height: 100%;
    outline: none;
}
.home-entry-link {
    position: absolute;
    left: calc(50% + 95px);
    top: calc(50% - 118px);
    transform: translate(-50%, -50%);
    display: block;
    width: 134px;
    height: 94px;
    border: none;
    background: transparent;
    color: transparent;
    text-decoration: none;
    cursor: pointer;
}
.home-entry-link:focus-visible {
    outline: 2px solid #0066cc;
}
.home-entry-link:hover {
    background: transparent;
}
.book-page {
    position: absolute;
    left: calc(50% - 26.5px);
    top: calc(50% + 3px);
    transform: translate(-50%, -50%) rotate(-6.875deg);
    width: 40px;
    height: 40px;
    text-align: center;
    color: #1a1c1f;
    font-family: Georgia, "Times New Roman", "Songti SC", "STSong", "SimSun", serif;
    cursor: pointer;
}
.book-title {
    display: -webkit-box;
    -webkit-line-clamp: 2;
    -webkit-box-orient: vertical;
    overflow: hidden;
    padding-top: 4px;
    font-size: 9px;
    font-weight: 600;
    line-height: 1.3;
    word-break: break-word;
}
.book-date {
    display: block;
    margin-top: 3px;
    font-size: 7px;
    line-height: 1.2;
    color: rgba(26, 28, 31, 0.7);
    white-space: nowrap;
}
`

const shellTemplate = `<!DOCTYPE html>
<html lang="{language}">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{page_title}</title>
    <link rel="canonical" href="{canonical}">
    <style>
{style}
    </style>
</head>
<body>
    <header>
        <h1><a href="/">{site_title}</a></h1>
    </header>

    <main>
{main}
    </main>

    <footer>
        &copy; {year} {site_title}. All rights reserved.
    </footer>
</body>
</html>
`

const indexTemplate = `<!DOCTYPE html>
<html lang="{language}">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{page_title}</title>
    <link rel="canonical" href="{canonical}">
    <style>
{style}
    </style>
</head>
<body class="home-page">
<main class="home-stage" aria-label="Stick figure holding a book">
    <canvas id="home-canvas" aria-hidden="true"></canvas>
    <a class="home-entry-link" href="/posts/" aria-label="Posts"><span class="book-page"><span class="book-title">{book_title}</span><span class="book-date">{book_date}</span></span></a>
</main>
<script src="/home.js" defer></script>
</body>
</html>
`

const labEntryTemplate = `<a class="lab-entry" href="{TCP_THROUGHPUT_ROUTE}">
    <span class="lab-entry-kicker">Interactive lab</span>
    <strong>TCP 吞吐量实验室</strong>
    <span>调节 ping，观察 TCP 滑动窗口与 HTTP POST 的传输速度 →</span>
</a>`

const postItemTemplate = `    <li class="post-item">
        <a href="{href}">{title}</a>
        <div class="post-meta">{date}</div>
        {tag_links}
        {lab_entry}
    </li>`

const labTemplate = `<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta name="description" content="调整 Ping / RTT，观察 TCP 滑动窗口和 HTTP POST 的传输速度曲线。">
    <title>TCP 吞吐量实验室 | {site_title}</title>
    <link rel="canonical" href="{canonical}">
    <link rel="stylesheet" href="/labs/tcp-throughput/lab.css">
</head>
<body>
    <main class="tcp-lab" id="tcp-lab">
        <figure class="throughput-figure">
            <svg id="throughput-chart" role="img" aria-label="Ping / RTT 对 TCP 滑动窗口和 HTTP POST 有效传输速度的影响折线图"></svg>
            <figcaption class="sr-only">拖动 Ping / RTT 选择框，查看 IW10 慢启动和 ACK 滑动窗口传输 8 MiB TCP 数据，以及相同连接上传 8 MiB HTTP POST 数据并等待响应的有效速度。</figcaption>
        </figure>

        <div class="chart-footer">
            <label class="ping-picker" for="rtt">
                <span>Ping / RTT</span>
                <input id="rtt" type="range" min="1" max="300" step="1" value="32">
                <output id="rtt-output" for="rtt">32 ms</output>
            </label>

            <div class="rtt-formula" aria-label="RTT 等于收到 Echo Reply 的时间减去发出 Echo Request 的时间">
                <span class="formula-name">RTT</span>
                <span>=</span>
                <span><i>t</i>(Echo Reply 收到)</span>
                <span>−</span>
                <span><i>t</i>(Echo Request 发出)</span>
            </div>
        </div>
    </main>
    <script src="/labs/tcp-throughput/model.js"></script>
    <script src="/labs/tcp-throughput/lab.js"></script>
</body>
</html>
`

const articleTemplate = `<article class="post-content">
    <a href="/posts/" class="back-link">&larr; Back to posts</a>
    <h1>{title}</h1>
    <div class="post-meta">{date}</div>
    {tag_links}
{body}
</article>`

const tagItemTemplate = `    <li class="post-item">
        <a href="{href}">{name}</a>
        <div class="post-meta">{count} post(s)</div>
    </li>`

const tagPostTemplate = `    <li class="post-item">
        <a href="{href}">{title}</a>
        <div class="post-meta">{date}</div>
    </li>`

const feedItemTemplate = `<item>
    <title>{title}</title>
    <link>{url}</link>
    <guid>{url}</guid>
    <pubDate>{date}</pubDate>
    <description>{description}</description>
</item>`

const feedTemplate = `<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
<channel>
    <title>{title}</title>
    <link>{base_url}</link>
    <description>{description}</description>
{items}
</channel>
</rss>
`

const sitemapTemplate = `<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
{urls}
</urlset>
`
