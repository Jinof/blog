package main

import "testing"

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
