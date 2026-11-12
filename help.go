package main

import (
	"fmt"
	"io"
)

const generalHelp = `Usage: go run . <command> [options]

Commands:
  build    Generate the static site in public/ (default command)
  serve    Build and preview the site locally
  new      Create a Markdown draft in posts/
  help     Show help, optionally for a command

Options:
  -h, --help    Show help without building or creating files

Examples:
  go run . build --draft
  go run . serve --draft --port 8080
  go run . new my-post --title "My Post" --tags "go,notes"
  go run . help new

Run commands from the repository root. Go 1.25 or newer is required.
`

func printHelp(output io.Writer, topic string) error {
	text := generalHelp
	switch topic {
	case "", "help", "-h", "--help":
	case "build":
		text = `Usage: go run . build [--draft]

Generate the static site in public/. Existing generated output is replaced.
  --draft       Include draft posts (hidden by default)
  -h, --help    Show help
`
	case "serve":
		text = `Usage: go run . serve [--draft] [--port PORT]

Build and preview the site at http://127.0.0.1:1313/.
  --draft       Include draft posts (hidden by default)
  --port PORT   Change the listening port (default: 1313)
  -h, --help    Show help

Posts, assets, and configuration changes rebuild every second.
Restart the preview after changing Go source.
`
	case "new":
		text = `Usage: go run . new <name> [--title TITLE] [--tags TAGS]

Create posts/<name>.md with the current local timestamp and draft: true.
Names may contain Chinese characters and subdirectories, relative to posts/.
Spaces become hyphens and names become lowercase. The .md suffix is optional.
Existing files are never overwritten.

  --title TITLE   Article title (default: filename with hyphens/underscores as spaces)
  --tags TAGS     Comma-separated tags (default: none)
  -h, --help      Show help without creating a post

Examples:
  go run . new hello-world
  go run . new notes/Go入门.md --title "Go 入门" --tags "go,学习"

Preview drafts with: go run . serve --draft
Set draft: false in the article when it is ready for publication.
`
	default:
		return fmt.Errorf("unknown help topic: %s. Run `go run . --help` for usage", topic)
	}
	_, err := fmt.Fprint(output, text)
	return err
}
