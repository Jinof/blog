package main

import (
	"bytes"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
	"time"
)

func TestHelpDoesNotRequireSiteOrWriteFiles(t *testing.T) {
	for _, args := range [][]string{{"--help"}, {"-h"}, {"help"}, {"help", "build"}, {"help", "serve"}, {"help", "new"}, {"build", "--help"}, {"serve", "-h"}, {"new", "--help"}, {"new", "example", "--help"}} {
		t.Run(strings.Join(args, " "), func(t *testing.T) {
			root := t.TempDir()
			var output bytes.Buffer
			if err := runWithOutput(root, args, &output); err != nil {
				t.Fatal(err)
			}
			if !strings.Contains(output.String(), "Usage:") {
				t.Fatal("missing usage", output.String())
			}
			entries, err := os.ReadDir(root)
			if err != nil || len(entries) != 0 {
				t.Fatalf("help modified site: %v %v", entries, err)
			}
		})
	}
	var output bytes.Buffer
	if err := runWithOutput(t.TempDir(), []string{"help", "missing"}, &output); err == nil {
		t.Fatal("unknown help topic accepted")
	}
}

func TestNewDraftRoundTrip(t *testing.T) {
	root := fixtureSite(t)
	now := time.Date(2026, 9, 17, 12, 34, 56, 0, time.FixedZone("CST", 8*3600))
	title := `Go "入门": C:\notes & <test>`
	path, err := createPost(root, "笔记/Hello World.md", title, "go, 中文 标签,go, ,quote\"tag", now)
	if err != nil {
		t.Fatal(err)
	}
	if path != filepath.Join(root, "posts/笔记/hello-world.md") {
		t.Fatal(path)
	}
	source := readFixture(t, root, "posts/笔记/hello-world.md")
	if !strings.Contains(source, "date: 2026-09-17T12:34:56+08:00\n") || !strings.HasSuffix(source, "---\n\n") {
		t.Fatal(source)
	}
	page, err := readPage(filepath.Join(root, "posts"), path)
	if err != nil {
		t.Fatal(err)
	}
	if page.Title != title || !page.Draft || page.Route != "/posts/笔记/hello-world/" || page.HTML != "" {
		t.Fatalf("unexpected draft: %+v", page)
	}
	if !reflect.DeepEqual(page.Tags, []string{"go", "中文 标签", `quote"tag`}) {
		t.Fatal(page.Tags)
	}
	if err := buildSite(root, false); err != nil {
		t.Fatal(err)
	}
	if _, err := os.Stat(filepath.Join(root, "public/", page.Route, "index.html")); !os.IsNotExist(err) {
		t.Fatal("new draft was published")
	}
	if err := buildSite(root, true); err != nil {
		t.Fatal(err)
	}
	if !strings.Contains(readFixture(t, root, "public/posts/笔记/hello-world/index.html"), escapeHTML(title)) {
		t.Fatal("new draft missing from preview")
	}
	if _, err := createPost(root, "笔记/Hello World.md", "replacement", "", now); err == nil {
		t.Fatal("overwrote existing draft")
	}
	if got := readFixture(t, root, "posts/笔记/hello-world.md"); got != source {
		t.Fatal("existing file changed")
	}
}

func TestNewCommandDefaultsAndFlags(t *testing.T) {
	for _, tc := range []struct {
		args        []string
		path, title string
	}{
		{[]string{"new", "Hello-World"}, "hello-world.md", "Hello World"},
		{[]string{"new", "My Notes.MD", "--title", "我的文章", "--tags", "go"}, "my-notes.md", "我的文章"},
		{[]string{"new", "--title", "标题", "中文"}, "中文.md", "标题"},
		{[]string{"new", "socket.io"}, "socket.io.md", "socket.io"},
	} {
		root := fixtureSite(t)
		var output bytes.Buffer
		if err := runWithOutput(root, tc.args, &output); err != nil {
			t.Fatal(err)
		}
		page, err := readPage(filepath.Join(root, "posts"), filepath.Join(root, "posts", tc.path))
		if err != nil || page.Title != tc.title || !page.Draft {
			t.Fatalf("%v: %+v %v", tc.args, page, err)
		}
		if !strings.Contains(output.String(), "Created draft") {
			t.Fatal("missing confirmation")
		}
	}
}

func TestNewRejectsInvalidArguments(t *testing.T) {
	for _, args := range [][]string{
		{"new"}, {"new", ""}, {"new", "one", "two"}, {"new", "one", "--unknown"},
		{"new", "one", "--title"}, {"new", "one", "--title", " "},
		{"new", "../outside"}, {"new", "/tmp/outside"}, {"new", "a/../outside"},
		{"new", "a//b"}, {"new", ".hidden"}, {"new", ".md"},
		{"new", "one", "--title", "line\nbreak"}, {"new", "one", "--tags", "line\nbreak"},
	} {
		t.Run(strings.Join(args, " "), func(t *testing.T) {
			root := fixtureSite(t)
			var output bytes.Buffer
			if err := runWithOutput(root, args, &output); err == nil {
				t.Fatal("invalid arguments accepted")
			}
			if _, err := os.Stat(filepath.Join(root, "posts")); !os.IsNotExist(err) {
				t.Fatal("invalid arguments created files")
			}
		})
	}
}

func TestNewCannotEscapeViaSymlink(t *testing.T) {
	root := fixtureSite(t)
	outside := t.TempDir()
	if err := os.Mkdir(filepath.Join(root, "posts"), 0755); err != nil {
		t.Fatal(err)
	}
	if err := os.Symlink(outside, filepath.Join(root, "posts/escape")); err != nil {
		t.Fatal(err)
	}
	if _, err := createPost(root, "escape/new-post", "", "", time.Now()); err == nil {
		t.Fatal("created file outside posts")
	}
	entries, err := os.ReadDir(outside)
	if err != nil || len(entries) != 0 {
		t.Fatal("outside directory modified")
	}
}
