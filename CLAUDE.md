# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a personal static blog deployed to Vercel (https://blog.jinof.vercel.app). The repository contains blog posts written in Markdown and uses a small Rust build tool with no crate dependencies.

## Commands

- **Build the site**: `cargo run -- build` - Generates static site files in `public/`
- **Build with drafts**: `cargo run -- build --draft`
- **Serve locally with drafts**: `cargo run -- serve --draft` - Builds the site and serves `public/` locally, rebuilding on source changes
- **Serve locally without drafts**: `cargo run -- serve` - Draft posts are hidden unless `--draft` is passed
- **Deploy**: `./delpoy.sh` - Builds locally, then pushes the current branch; Vercel runs `cargo run --release -- build`

## Code Structure

- `site.config.json` - Main site configuration (baseURL, language, title)
- `src/main.rs` - Dependency-free Rust build and local preview tool
- `assets/home.js` - Dependency-free Canvas 2D homepage scene
- `/` - Canvas 2D homepage with a link to `/posts/`
- `/posts/` - Post index page for Markdown posts
- `posts/` - All Markdown posts included by the Rust build tool
- `assets/` - New static assets copied into `public/`
- `public/` - Generated static site output, ignored by git

## Blog Post Format

Posts in `posts/` use frontmatter with fields like title, date, draft, and tags.
