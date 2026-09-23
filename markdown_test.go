package main

import (
	"strings"
	"testing"
)

func TestMarkdownCompatibility(t *testing.T) {
	cases := []struct{ name, source, want string }{
		{"paragraph", "first\nsecond\n\nlast", "<p>first second</p>\n<p>last</p>"},
		{"heading", "## 中文 [链接](https://example.com)", `<h2 id="中文-链接">中文 <a href="https://example.com">链接</a></h2>`},
		{"lists", "- one\n* two\n1. first\n2. second", "<ul>\n  <li>one</li>\n  <li>two</li>\n</ul>\n<ol>\n  <li>first</li>\n  <li>second</li>\n</ol>"},
		{"fence", "```go\nif a < b {\n}\n```", "<pre><code class=\"language-go\">if a &lt; b {\n}</code></pre>"},
		{"unclosed fence", "~~~go\n<&", "<pre><code>&lt;&amp;</code></pre>"},
		{"indented code", "    one\n        two", "<pre><code>one\ntwo</code></pre>"},
		{"quote", "> quoted\n>\n> next", "<blockquote>\n<p>quoted</p>\n<p>next</p>\n</blockquote>"},
		{"escaping", "<script> & \"value\" `a<b`", "<p>&lt;script&gt; &amp; &quot;value&quot; <code>a&lt;b</code></p>"},
		{"links", `[local](./docs/a) ![a'b](./image.png) [anchor](#中文)`, `<p><a href="/docs/a">local</a> <img src="/image.png" alt="a&#39;b"> <a href="#中文">anchor</a></p>`},
		{"malformed", "[text](unfinished `code", "<p>[text](unfinished `code</p>"},
		{"rules", "---\n***", "<hr>\n<hr>"},
		{"unsupported syntax", "**bold**\n\n| a | b |", "<p>**bold**</p>\n<p>| a | b |</p>"},
		{"crlf", "# title\r\n\r\ntext\r\n", "<h1 id=\"title\">title</h1>\n<p>text</p>"},
	}
	for _, tc := range cases {
		t.Run(tc.name, func(t *testing.T) {
			if got := markdownToHTML(tc.source); got != tc.want {
				t.Fatalf("got  %q\nwant %q", got, tc.want)
			}
		})
	}
}

func TestMarkdownTables(t *testing.T) {
	source := "intro\n\n| 模型 | 内容 | 数量 |\n| :--- | :---: | ---: |\n| Qwen | `a\\|b` [原图](./image.png) | 3 |\n| Gemma | <script>alert(1)</script> |\n| Extra | value | 2 | ignored |\n\nafter"
	got := markdownToHTML(source)
	for _, want := range []string{
		`<th scope="col" style="text-align:left">模型</th>`,
		`<th scope="col" style="text-align:center">内容</th>`,
		`<th scope="col" style="text-align:right">数量</th>`,
		`<code>a|b</code> <a href="/image.png">原图</a>`,
		`&lt;script&gt;alert(1)&lt;/script&gt;`,
		`<td style="text-align:right"></td>`,
		"</table></div>\n<p>after</p>",
	} {
		if !strings.Contains(got, want) {
			t.Errorf("missing %q in %s", want, got)
		}
	}
	if strings.Contains(got, "<script>") || strings.Contains(got, "ignored") || strings.Count(got, "<td ") != 9 {
		t.Fatalf("unsafe or malformed table: %s", got)
	}
}

func TestMarkdownTableRecognition(t *testing.T) {
	for _, source := range []string{
		"A | B\n--- | ---\none | two",
		"| A |\n| --- |\n| one |",
	} {
		if !strings.Contains(markdownToHTML(source), "<table>") {
			t.Errorf("table not recognized: %q", source)
		}
	}
	for _, source := range []string{
		"A | B\n--- | invalid\none | two",
		"A | B\n--- | --- | ---\none | two",
		"A | B\n::--- | ---\none | two",
		"```text\n| A | B |\n| --- | --- |\n```",
		"    | A | B |\n    | --- | --- |",
	} {
		if strings.Contains(markdownToHTML(source), "<table>") {
			t.Errorf("non-table changed: %q", source)
		}
	}
}
