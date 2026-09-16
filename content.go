package main

import (
	"encoding/json"
	"fmt"
	"io/fs"
	"os"
	"path/filepath"
	"sort"
	"strings"
	"unicode/utf8"
)

type Config struct {
	BaseURL      string `json:"baseURL"`
	LanguageCode string `json:"languageCode"`
	Title        string `json:"title"`
	Description  string `json:"description"`
}

type Page struct {
	Route, Title, DateText, SortKey string
	Draft                           bool
	Tags                            []string
	HTML                            string
}

type TagPage struct {
	Name, Route string
	Pages       []Page
}
type Frontmatter struct {
	Values map[string]string
	Lists  map[string][]string
}

func readConfig(path string) (Config, error) {
	config := Config{"https://blog.jinof.vercel.app", "en-us", "Jinof's Blog", "Jinof's Blog"}
	data, err := os.ReadFile(path)
	if err != nil {
		return config, fmt.Errorf("read site.config.json: %w", err)
	}
	if err := json.Unmarshal(data, &config); err != nil {
		return config, fmt.Errorf("parse site.config.json: %w", err)
	}
	return config, nil
}

func readSitePages(root string) ([]Page, error) {
	contentDir := filepath.Join(root, "posts")
	byRoute := map[string]Page{}
	err := filepath.WalkDir(contentDir, func(path string, entry fs.DirEntry, err error) error {
		if os.IsNotExist(err) && path == contentDir {
			return nil
		}
		if err != nil {
			return err
		}
		if entry.IsDir() || !isMarkdownDocument(path) {
			return nil
		}
		page, err := readPage(contentDir, path)
		if err != nil {
			return err
		}
		byRoute[page.Route] = page
		return nil
	})
	if err != nil {
		return nil, err
	}
	pages := make([]Page, 0, len(byRoute))
	for _, page := range byRoute {
		pages = append(pages, page)
	}
	sort.Slice(pages, func(i, j int) bool { return pages[i].Route < pages[j].Route })
	// Rust sorts dates stably after collecting pages in route order.
	sort.SliceStable(pages, func(i, j int) bool { return pages[i].SortKey > pages[j].SortKey })
	return pages, nil
}

func isMarkdownDocument(path string) bool {
	if strings.HasPrefix(filepath.Base(path), ".") {
		return false
	}
	ext := filepath.Ext(path)
	return ext == ".md" || ext == ".markdown"
}

func readPage(contentDir, file string) (Page, error) {
	data, err := os.ReadFile(file)
	if err != nil {
		return Page{}, fmt.Errorf("read %s: %w", file, err)
	}
	if !utf8.Valid(data) {
		return Page{}, fmt.Errorf("read %s: invalid UTF-8", file)
	}
	frontmatter, body := parseFrontmatter(string(data))
	title, ok := frontmatter.Values["title"]
	if !ok {
		title = strings.NewReplacer("-", " ", "_", " ").Replace(strings.TrimSuffix(filepath.Base(file), filepath.Ext(file)))
	}
	date := frontmatter.Values["date"]
	dateText := ""
	if len(date) >= 10 && utf8.ValidString(date[:10]) {
		dateText = date[:10]
	}
	sortKey := date
	if sortKey == "" {
		sortKey = "0000-00-00"
	}
	route, err := routeFor(contentDir, file)
	if err != nil {
		return Page{}, err
	}
	return Page{route, title, dateText, sortKey, frontmatter.Values["draft"] == "true", frontmatter.Lists["tags"], markdownToHTML(strings.TrimSpace(body))}, nil
}

func parseFrontmatter(source string) (Frontmatter, string) {
	normalized := strings.ReplaceAll(source, "\r\n", "\n")
	lines := sourceLines(normalized)
	frontmatter := Frontmatter{map[string]string{}, map[string][]string{}}
	if len(lines) == 0 || lines[0] != "---" {
		return frontmatter, normalized
	}
	end := -1
	for i := 1; i < len(lines); i++ {
		if lines[i] == "---" {
			end = i
			break
		}
	}
	if end < 0 {
		return frontmatter, normalized
	}
	currentKey := ""
	for _, line := range lines[1:end] {
		trimmed := strings.TrimSpace(line)
		if item, ok := strings.CutPrefix(trimmed, "- "); ok {
			if currentKey != "" {
				frontmatter.Lists[currentKey] = append(frontmatter.Lists[currentKey], parseScalar(item))
			}
			continue
		}
		if key, value, ok := strings.Cut(trimmed, ":"); ok {
			currentKey = strings.TrimSpace(key)
			value = strings.TrimSpace(value)
			if value == "" {
				if _, ok := frontmatter.Lists[currentKey]; !ok {
					frontmatter.Lists[currentKey] = nil
				}
			} else {
				frontmatter.Values[currentKey] = parseScalar(value)
			}
		}
	}
	return frontmatter, strings.Join(lines[end+1:], "\n")
}

func parseScalar(value string) string {
	value = strings.TrimSpace(value)
	if len(value) >= 2 && ((value[0] == '"' && value[len(value)-1] == '"') || (value[0] == '\'' && value[len(value)-1] == '\'')) {
		return value[1 : len(value)-1]
	}
	return value
}

func routeFor(contentDir, file string) (string, error) {
	relative, err := filepath.Rel(contentDir, file)
	if err != nil {
		return "", err
	}
	relative = strings.TrimSuffix(relative, filepath.Ext(relative))
	parts := strings.Split(filepath.ToSlash(relative), "/")
	for i := range parts {
		parts[i] = slugify(parts[i])
	}
	return "/posts/" + strings.Join(parts, "/") + "/", nil
}

func slugify(value string) string {
	value = strings.NewReplacer("\"", "", "'", "").Replace(strings.ToLower(strings.TrimSpace(value)))
	return strings.Join(strings.Fields(value), "-")
}

func collectTags(pages []Page) []TagPage {
	names := map[string]bool{}
	for _, page := range pages {
		for _, tag := range page.Tags {
			names[tag] = true
		}
	}
	sorted := make([]string, 0, len(names))
	for name := range names {
		sorted = append(sorted, name)
	}
	sort.Strings(sorted)
	tags := make([]TagPage, 0, len(sorted))
	for _, name := range sorted {
		tag := TagPage{Name: name, Route: "/tags/" + slugify(name) + "/"}
		for _, page := range pages {
			for _, value := range page.Tags {
				if value == name {
					tag.Pages = append(tag.Pages, page)
					break
				}
			}
		}
		tags = append(tags, tag)
	}
	return tags
}
