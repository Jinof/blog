# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a personal static blog deployed to Vercel (https://blog.jinof.vercel.app). The repository contains blog posts written in Markdown and uses a small Go build tool using only the standard library (Go 1.25 or newer).

## Commands

- **Build the site**: `go run . build` - Generates static site files in `public/`
- **Build with drafts**: `go run . build --draft`
- **Serve locally with drafts**: `go run . serve --draft` - Builds the site and serves `public/` locally, rebuilding on source changes
- **Serve locally without drafts**: `go run . serve` - Draft posts are hidden unless `--draft` is passed
- **Deploy**: `./delpoy.sh` - Builds locally, then pushes `master` to `origin`; Vercel runs `go run . build`

Run commands from the repository root. With no subcommand, `go run .` builds the site. Preview binds to `127.0.0.1:1313`; use `--port 8080` to change the port. Posts, assets, and configuration changes rebuild automatically every second. Restart the preview after changing Go source.

- **Validate**: `go test -race ./...`, `go vet ./...`, and `node tests/tcp_throughput_model.test.js`
- **Build a binary**: `go build -o bin/jinof-blog .`

The independent TCP experiment under `experiments/tcp-http/` retains its Rust router and separate Go server module.

## Code Structure

- `site.config.json` - Main site configuration (baseURL, language, title)
- Root Go files - Command entrypoint, content/Markdown parsing, HTML templates, rendering, build, and local preview
- `assets/home.js` - Dependency-free Canvas 2D homepage scene
- `/` - Canvas 2D homepage with a link to `/posts/`
- `/posts/` - Post index page for Markdown posts
- `posts/` - All Markdown posts included by the Go build tool
- `assets/` - New static assets copied into `public/`
- `public/` - Generated static site output, ignored by git

## Blog Post Format

Posts in `posts/` use frontmatter with fields like title, date, draft, and tags.
