package main

import (
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
	"time"
	"unicode"
)

func newCommand(root string, args []string, output io.Writer) error {
	flags := flag.NewFlagSet("new", flag.ContinueOnError)
	flags.SetOutput(io.Discard)
	title := flags.String("title", "", "Article title")
	tags := flags.String("tags", "", "Comma-separated tags")
	// Accept the documented name-first form, as well as flags before the name.
	name := ""
	if len(args) > 0 && !strings.HasPrefix(args[0], "-") {
		name, args = args[0], args[1:]
	}
	if err := flags.Parse(args); err != nil {
		if errors.Is(err, flag.ErrHelp) {
			return printHelp(output, "new")
		}
		return fmt.Errorf("new: %w. Run `go run . help new` for usage", err)
	}
	if name == "" && flags.NArg() == 1 {
		name = flags.Arg(0)
	} else if flags.NArg() != 0 {
		return fmt.Errorf("new expects one name. Run `go run . help new` for usage")
	}
	if name == "" {
		return fmt.Errorf("new requires an article name. Run `go run . help new` for usage")
	}
	titleSet := false
	flags.Visit(func(f *flag.Flag) {
		if f.Name == "title" {
			titleSet = true
		}
	})
	if titleSet && strings.TrimSpace(*title) == "" {
		return fmt.Errorf("article title must not be empty")
	}
	path, err := createPost(root, name, *title, *tags, time.Now())
	if err != nil {
		return err
	}
	_, err = fmt.Fprintf(output, "Created draft %s\nPreview with: go run . serve --draft\n", path)
	return err
}

func newPostPath(name string) (string, error) {
	if strings.TrimSpace(name) == "" || filepath.IsAbs(name) || strings.Contains(name, "\\") {
		return "", fmt.Errorf("article name must be a relative path inside posts/")
	}
	parts := strings.Split(name, "/")
	for i, part := range parts {
		if i == len(parts)-1 {
			if strings.EqualFold(filepath.Ext(part), ".md") {
				part = part[:len(part)-3]
			}
		}
		part = slugify(part)
		if part == "" || strings.HasPrefix(part, ".") || strings.ContainsAny(part, "/\\") || strings.ContainsFunc(part, unicode.IsControl) {
			return "", fmt.Errorf("invalid article path component %q", parts[i])
		}
		parts[i] = part
	}
	return filepath.Join(parts...) + ".md", nil
}

// JSON string quoting is valid YAML and preserves quotes/backslashes in titles.
func frontmatterString(value string) string {
	data, _ := json.Marshal(value)
	return string(data)
}

func createPost(root, name, title, tagList string, now time.Time) (string, error) {
	relative, err := newPostPath(name)
	if err != nil {
		return "", err
	}
	if _, err := readConfig(filepath.Join(root, "site.config.json")); err != nil {
		return "", err
	}
	if title == "" {
		stem := strings.TrimSuffix(filepath.Base(name), filepath.Ext(name))
		// Only .md is an optional extension; dots can be part of the article name.
		if !strings.EqualFold(filepath.Ext(name), ".md") {
			stem = filepath.Base(name)
		}
		title = strings.NewReplacer("-", " ", "_", " ").Replace(stem)
	}
	if strings.ContainsFunc(title, unicode.IsControl) {
		return "", fmt.Errorf("article title must be a single line without control characters")
	}
	var tags []string
	seen := map[string]bool{}
	for _, tag := range strings.Split(tagList, ",") {
		tag = strings.TrimSpace(tag)
		if tag == "" || seen[tag] {
			continue
		}
		if strings.ContainsFunc(tag, unicode.IsControl) {
			return "", fmt.Errorf("tags must not contain control characters")
		}
		tags = append(tags, tag)
		seen[tag] = true
	}
	var body strings.Builder
	fmt.Fprintf(&body, "---\ntitle: %s\ndate: %s\ndraft: true\ntags:\n", frontmatterString(title), now.Format(time.RFC3339))
	for _, tag := range tags {
		fmt.Fprintf(&body, "  - %s\n", frontmatterString(tag))
	}
	body.WriteString("---\n\n")
	// Root confines even symlinked subdirectories to the repository/posts tree.
	repository, err := os.OpenRoot(root)
	if err != nil {
		return "", err
	}
	defer repository.Close()
	if err := repository.MkdirAll("posts", 0755); err != nil {
		return "", err
	}
	posts, err := repository.OpenRoot("posts")
	if err != nil {
		return "", err
	}
	defer posts.Close()
	if err := posts.MkdirAll(filepath.Dir(relative), 0755); err != nil {
		return "", err
	}
	file, err := posts.OpenFile(relative, os.O_WRONLY|os.O_CREATE|os.O_EXCL, 0644)
	if err != nil {
		if errors.Is(err, os.ErrExist) {
			return "", fmt.Errorf("article already exists: posts/%s", filepath.ToSlash(relative))
		}
		return "", fmt.Errorf("create article: %w", err)
	}
	_, writeErr := io.WriteString(file, body.String())
	closeErr := file.Close()
	if err := errors.Join(writeErr, closeErr); err != nil {
		return "", fmt.Errorf("write article posts/%s: %w", filepath.ToSlash(relative), err)
	}
	return filepath.Join(root, "posts", relative), nil
}
