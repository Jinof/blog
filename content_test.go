package main

import (
	"encoding/xml"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

func writeFixture(t *testing.T, root, name, contents string) {
	t.Helper()
	path := filepath.Join(root, name)
	if err := os.MkdirAll(filepath.Dir(path), 0755); err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(path, []byte(contents), 0644); err != nil {
		t.Fatal(err)
	}
}

func fixtureSite(t *testing.T) string {
	t.Helper()
	root := t.TempDir()
	writeFixture(t, root, "site.config.json", `{"baseURL":"https://example.com/","title":"Test & Blog","languageCode":"zh-CN"}`)
	return root
}

func readFixture(t *testing.T, root, name string) string {
	t.Helper()
	data, err := os.ReadFile(filepath.Join(root, name))
	if err != nil {
		t.Fatal(err)
	}
	return string(data)
}

func TestFrontmatterAndRoutes(t *testing.T) {
	source := "---\r\ntitle: '中文: 标题'\r\ndate: 2026-09-16T08:00:00Z\r\ndraft: true\r\ntags:\r\n - Go\r\n - '中文 标签'\r\n---\r\n# 正文\r\n"
	front, body := parseFrontmatter(source)
	if front.Values["title"] != "中文: 标题" || front.Values["draft"] != "true" || body != "# 正文" {
		t.Fatalf("unexpected frontmatter/body: %+v %q", front, body)
	}
	if !reflect.DeepEqual(front.Lists["tags"], []string{"Go", "中文 标签"}) {
		t.Fatal(front.Lists)
	}
	root := fixtureSite(t)
	writeFixture(t, root, "posts/子目录/My 中文.md", source)
	page, err := readPage(filepath.Join(root, "posts"), filepath.Join(root, "posts/子目录/My 中文.md"))
	if err != nil {
		t.Fatal(err)
	}
	if page.Route != "/posts/子目录/my-中文/" || page.DateText != "2026-09-16" || !page.Draft {
		t.Fatalf("unexpected page: %+v", page)
	}
	if got := encodeURI(page.Route); got != "/posts/%E5%AD%90%E7%9B%AE%E5%BD%95/my-%E4%B8%AD%E6%96%87/" {
		t.Fatal(got)
	}
	for _, source := range []string{"plain\r\ntext", "---\ntitle: unclosed\nbody"} {
		front, body := parseFrontmatter(source)
		if len(front.Values) != 0 || body != strings.ReplaceAll(source, "\r\n", "\n") {
			t.Fatalf("unterminated header changed: %q", body)
		}
	}
}

func TestBuildOrderingDraftsTagsAndAssets(t *testing.T) {
	root := fixtureSite(t)
	for name, body := range map[string]string{
		"b.md":       "---\ntitle: Second\ndate: 2026-09-16\ntags:\n - Go\n---\nB",
		"a.markdown": "---\ntitle: First\ndate: 2026-09-16\ntags:\n - Go\n - 中文 标签\n---\nA",
		"draft.md":   "---\ntitle: Secret\ndate: 2026-09-17\ndraft: true\ntags:\n - Secret\n---\nDraft",
		"old.md":     "---\ntitle: Old\ndate: 2020-01-01\n---\nOld",
		".hidden.md": "Hidden",
	} {
		writeFixture(t, root, "posts/"+name, body)
	}
	writeFixture(t, root, "posts/images/picture.PNG", "picture")
	writeFixture(t, root, "posts/ignored.bin", "excluded")
	writeFixture(t, root, "assets/custom.bin", "global")
	writeFixture(t, root, "assets/.gitkeep", "excluded")
	writeFixture(t, root, "public/stale.txt", "stale")
	if err := buildSite(root, false); err != nil {
		t.Fatal(err)
	}
	index := readFixture(t, root, "public/posts/index.html")
	first, second, old := strings.Index(index, ">First</a>"), strings.Index(index, ">Second</a>"), strings.Index(index, ">Old</a>")
	if !(first >= 0 && first < second && second < old) {
		t.Fatal("date and route ordering changed")
	}
	for _, name := range []string{"posts/draft/index.html", "tags/secret/index.html", "posts/.hidden/index.html", "ignored.bin", ".gitkeep", "stale.txt"} {
		if _, err := os.Stat(filepath.Join(root, "public", name)); !os.IsNotExist(err) {
			t.Fatalf("unexpected output %s: %v", name, err)
		}
	}
	if readFixture(t, root, "public/images/picture.PNG") != "picture" || readFixture(t, root, "public/custom.bin") != "global" {
		t.Fatal("asset copy changed")
	}
	for _, name := range []string{"public/index.xml", "public/sitemap.xml", "public/posts/index.html", "public/tags/index.html"} {
		if strings.Contains(readFixture(t, root, name), "Secret") {
			t.Fatalf("draft leaked into %s", name)
		}
	}
	tagPage := readFixture(t, root, "public/tags/go/index.html")
	if !strings.Contains(tagPage, ">First</a>") || !strings.Contains(tagPage, ">Second</a>") {
		t.Fatal("missing tag members")
	}
	if err := buildSite(root, true); err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(readFixture(t, root, "public/index.html"), "Secret") {
		t.Fatal("latest draft absent from homepage")
	}
	if !strings.Contains(readFixture(t, root, "public/tags/secret/index.html"), "Secret") {
		t.Fatal("draft tag absent")
	}
	if err := buildSite(root, false); err != nil {
		t.Fatal(err)
	}
	if _, err := os.Stat(filepath.Join(root, "public/posts/draft/index.html")); !os.IsNotExist(err) {
		t.Fatal("draft survives normal rebuild")
	}
}

func TestEmptySiteConfigAndErrors(t *testing.T) {
	root := fixtureSite(t)
	if err := run(root, nil); err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(readFixture(t, root, "public/posts/index.html"), "No posts yet.") {
		t.Fatal("missing empty state")
	}
	config, err := readConfig(filepath.Join(root, "site.config.json"))
	if err != nil || config.Description != "Jinof's Blog" {
		t.Fatalf("defaults: %+v %v", config, err)
	}
	if err := run(root, []string{"unknown"}); err == nil {
		t.Fatal("unknown command succeeded")
	}
	if err := buildSite(t.TempDir(), false); err == nil || !strings.Contains(err.Error(), "site.config.json") {
		t.Fatalf("missing configuration: %v", err)
	}
	writeFixture(t, root, "site.config.json", "invalid")
	if err := buildSite(root, false); err == nil {
		t.Fatal("invalid config accepted")
	}
	if !strings.Contains(readFixture(t, root, "public/index.html"), "home-canvas") {
		t.Fatal("config error destroyed output")
	}
}

func TestFeedUnicodeLimitAndEscaping(t *testing.T) {
	config := Config{BaseURL: "https://example.com/", Title: `Blog & "Title"`, Description: "A < B"}
	pages := make([]Page, 21)
	for i := range pages {
		pages[i] = Page{Title: "中文 & '文章'", Route: "/posts/中文/", DateText: "2026-09-16", HTML: "<p>" + strings.Repeat("中", 501) + "</p>"}
	}
	feed := renderFeed(config, pages)
	var parsed struct {
		Channel struct {
			Items []struct {
				Description string `xml:"description"`
				Title       string `xml:"title"`
				Link        string `xml:"link"`
			} `xml:"item"`
		} `xml:"channel"`
	}
	if err := xml.Unmarshal([]byte(feed), &parsed); err != nil {
		t.Fatal(err)
	}
	if len(parsed.Channel.Items) != 20 {
		t.Fatal("RSS limit changed")
	}
	item := parsed.Channel.Items[0]
	if len([]rune(item.Description)) != 500 || item.Title != "中文 & '文章'" || item.Link != "https://example.com/posts/%E4%B8%AD%E6%96%87/" {
		t.Fatalf("invalid feed item: %+v", item)
	}
	if got := renderTemplate("{main} {year}", "{main}", "{year}", "{year}", "2026"); got != "{year} 2026" {
		t.Fatal("inserted content recursively substituted")
	}
}
