package main

import (
	"context"
	"fmt"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"testing"
	"time"
)

func TestPreviewAddressAndDraftFlag(t *testing.T) {
	if previewAddress(nil) != "127.0.0.1:1313" {
		t.Fatal("default address changed")
	}
	if previewAddress([]string{"serve", "--draft", "--port", "8765"}) != "127.0.0.1:8765" {
		t.Fatal("custom address ignored")
	}
	if !includeDraftPosts([]string{"build", "--draft"}) || includeDraftPosts([]string{"serve"}) {
		t.Fatal("draft flag changed")
	}
}

func TestPreviewHTTP(t *testing.T) {
	root := fixtureSite(t)
	writeFixture(t, root, "posts/中文.md", "# 中文")
	writeFixture(t, root, "posts/100%25.md", "literal percent")
	writeFixture(t, root, "assets/home.js", "// script")
	p, err := newPreview(root, false)
	if err != nil {
		t.Fatal(err)
	}
	for _, tc := range []struct {
		path       string
		status     int
		mime, body string
	}{
		{"/", 200, "text/html; charset=utf-8", "home-canvas"},
		{"/posts", 200, "text/html; charset=utf-8", "中文"},
		{"/posts/%E4%B8%AD%E6%96%87/?q=test", 200, "text/html; charset=utf-8", "中文"},
		{"/posts/100%2525/", 200, "text/html; charset=utf-8", "literal percent"},
		{"/home.js", 200, "text/javascript; charset=utf-8", "// script"},
		{"/index.xml", 200, "application/xml; charset=utf-8", "<rss"},
		{"/missing", 404, "text/plain; charset=utf-8", "Not found"},
		{"/%2e%2e/site.config.json", 404, "text/plain; charset=utf-8", "Not found"},
	} {
		t.Run(tc.path, func(t *testing.T) {
			response := httptest.NewRecorder()
			p.ServeHTTP(response, httptest.NewRequest("GET", tc.path, nil))
			if response.Code != tc.status || response.Header().Get("Content-Type") != tc.mime || !strings.Contains(response.Body.String(), tc.body) {
				t.Fatalf("response %d %s %s", response.Code, response.Header(), response.Body.String())
			}
		})
	}
	response := httptest.NewRecorder()
	p.ServeHTTP(response, httptest.NewRequest("HEAD", "/", nil))
	if response.Code != 200 || response.Body.Len() != 0 || response.Header().Get("Content-Length") == "" {
		t.Fatal("HEAD response invalid")
	}
}

func TestPollingRebuildsAndRecovers(t *testing.T) {
	root := fixtureSite(t)
	writeFixture(t, root, "posts/a.md", "original")
	writeFixture(t, root, "assets/test.txt", "old")
	p, err := newPreview(root, true)
	if err != nil {
		t.Fatal(err)
	}
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	done := make(chan struct{})
	go func() { defer close(done); p.poll(ctx, 10*time.Millisecond) }()
	defer func() { cancel(); <-done }()
	await := func(path, want string, status int) {
		t.Helper()
		deadline := time.Now().Add(3 * time.Second)
		for time.Now().Before(deadline) {
			response := httptest.NewRecorder()
			p.ServeHTTP(response, httptest.NewRequest("GET", path, nil))
			if response.Code == status && strings.Contains(response.Body.String(), want) {
				return
			}
			time.Sleep(10 * time.Millisecond)
		}
		t.Fatalf("rebuild did not produce %s (%d, %q)", path, status, want)
	}
	writeFixture(t, root, "posts/new.md", "---\ndraft: true\n---\nnew draft")
	await("/posts/new/", "new draft", 200)
	writeFixture(t, root, "posts/a.md", "modified content")
	await("/posts/a/", "modified content", 200)
	if err := os.Remove(filepath.Join(root, "posts/a.md")); err != nil {
		t.Fatal(err)
	}
	await("/posts/a/", "Not found", 404)
	writeFixture(t, root, "assets/test.txt", "updated asset")
	await("/test.txt", "updated asset", 200)
	writeFixture(t, root, "site.config.json", `{"title":"Changed title"}`)
	await("/", "Changed title", 200)
	// A failed rebuild must not advance the signature or discard readable output.
	cancel()
	<-done
	writeFixture(t, root, "site.config.json", "invalid")
	previous := p.previous
	if err := p.rebuildIfChanged(); err == nil {
		t.Fatal("bad config accepted")
	}
	if p.previous != previous {
		t.Fatal("failed build advanced signature")
	}
	await("/", "Changed title", 200)
	writeFixture(t, root, "site.config.json", `{"title":"Recovered"}`)
	if err := p.rebuildIfChanged(); err != nil {
		t.Fatal(err)
	}
	await("/", "Recovered", 200)
}

func TestConcurrentPreviewRebuild(t *testing.T) {
	root := fixtureSite(t)
	writeFixture(t, root, "posts/a.md", "content")
	p, err := newPreview(root, false)
	if err != nil {
		t.Fatal(err)
	}
	var wg sync.WaitGroup
	for range 4 {
		wg.Add(1)
		go func() {
			defer wg.Done()
			for range 30 {
				response := httptest.NewRecorder()
				p.ServeHTTP(response, httptest.NewRequest("GET", "/posts/a/", nil))
				if response.Code != 200 || !strings.Contains(response.Body.String(), "</html>") {
					t.Errorf("incomplete response: %d", response.Code)
					return
				}
			}
		}()
	}
	for i := range 5 {
		writeFixture(t, root, "posts/a.md", fmt.Sprintf("updated content %d", i))
		if err := p.rebuildIfChanged(); err != nil {
			t.Error(err)
		}
	}
	wg.Wait()
}
