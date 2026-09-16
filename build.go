package main

import (
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"strings"
)

func buildSite(root string, includeDrafts bool) error {
	config, err := readConfig(filepath.Join(root, "site.config.json"))
	if err != nil {
		return err
	}
	allPages, err := readSitePages(root)
	if err != nil {
		return err
	}
	pages := make([]Page, 0, len(allPages))
	for _, page := range allPages {
		if includeDrafts || !page.Draft {
			pages = append(pages, page)
		}
	}
	tags := collectTags(pages)
	publicDir := filepath.Join(root, "public")
	if err := os.RemoveAll(publicDir); err != nil {
		return fmt.Errorf("remove public/: %w", err)
	}
	if err := os.MkdirAll(publicDir, 0755); err != nil {
		return err
	}
	if err := copyFiltered(filepath.Join(root, "posts"), publicDir, isPostAsset); err != nil {
		return err
	}
	if err := copyFiltered(filepath.Join(root, "assets"), publicDir, func(path string) bool { return filepath.Base(path) != ".gitkeep" }); err != nil {
		return err
	}
	for _, page := range pages {
		if err := writePublicFile(publicDir, page.Route+"index.html", renderArticle(config, page)); err != nil {
			return err
		}
	}
	outputs := []struct{ route, content string }{
		{"/index.html", renderIndex(config, pages)},
		{"/posts/index.html", renderPostIndex(config, pages)},
		{"/index.xml", renderFeed(config, pages)},
		{"/sitemap.xml", renderSitemap(config, pages, tags)},
		{"/tags/index.html", renderTagIndex(config, tags)},
		{tcpThroughputRoute + "index.html", renderTCPThroughputLab(config)},
	}
	for _, output := range outputs {
		if err := writePublicFile(publicDir, output.route, output.content); err != nil {
			return err
		}
	}
	for _, tag := range tags {
		if err := writePublicFile(publicDir, tag.Route+"index.html", renderTagPage(config, tag)); err != nil {
			return err
		}
	}
	if includeDrafts {
		fmt.Printf("Built %d page(s) into public/ including drafts.\n", len(pages))
	} else {
		fmt.Printf("Built %d page(s) into public/.\n", len(pages))
	}
	return nil
}

func isPostAsset(path string) bool {
	switch strings.ToLower(filepath.Ext(path)) {
	case ".apng", ".avif", ".css", ".gif", ".ico", ".jpeg", ".jpg", ".js", ".pdf", ".png", ".svg", ".txt", ".webp", ".xml":
		return true
	default:
		return false
	}
}

func copyFiltered(source, destination string, shouldCopy func(string) bool) error {
	return filepath.WalkDir(source, func(path string, entry fs.DirEntry, err error) error {
		if os.IsNotExist(err) && path == source {
			return nil
		}
		if err != nil {
			return err
		}
		relative, err := filepath.Rel(source, path)
		if err != nil {
			return err
		}
		target := filepath.Join(destination, relative)
		if entry.IsDir() {
			return os.MkdirAll(target, 0755)
		}
		if !shouldCopy(path) {
			return nil
		}
		contents, err := os.ReadFile(path)
		if err != nil {
			return fmt.Errorf("copy %s: %w", path, err)
		}
		return os.WriteFile(target, contents, 0644)
	})
}

func writePublicFile(publicDir, route, contents string) error {
	target := filepath.Join(publicDir, filepath.FromSlash(strings.TrimLeft(route, "/")))
	if err := os.MkdirAll(filepath.Dir(target), 0755); err != nil {
		return err
	}
	if err := os.WriteFile(target, []byte(contents), 0644); err != nil {
		return fmt.Errorf("write %s: %w", target, err)
	}
	return nil
}
