# AGENTS.md

This file provides guidance to Codex (Codex.ai/code) when working with code in this repository.

## Project Overview

This is a personal static blog deployed to Vercel (https://blog.jinof.vercel.app). The repository contains blog posts written in Markdown and uses a small Rust build tool with no crate dependencies.

## Commands

- **Local prerequisites for Bevy/WebAssembly builds**:
  - `rustup target add wasm32-unknown-unknown`
  - `cargo install wasm-bindgen-cli --locked`
- **Build the site**: `cargo run -- build` - Generates static site files in `public/`
- **Build with drafts**: `cargo run -- build --draft`
- **Serve locally with drafts**: `cargo run -- serve --draft` - Builds the site and serves `public/` locally, rebuilding on source changes
- **Serve locally without drafts**: `cargo run -- serve` - Draft posts are hidden unless `--draft` is passed
- **Deploy**: `./delpoy.sh` - Builds locally, then pushes the current branch; Vercel runs `cargo run --release -- build`

## Code Structure

- `site.config.json` - Main site configuration (baseURL, language, title)
- `src/main.rs` - Dependency-free Rust build and local preview tool
- `src/bin/home_bevy.rs` - Bevy/WebAssembly homepage scene
- `/` - Animation-only Bevy homepage with a link to `/posts/`
- `/posts/` - Post index page for new Markdown posts
- `posts/` - New Markdown posts included by the Rust build tool
- `assets/` - New static assets copied into `public/`
- `content/` - Legacy Hugo-era Markdown archive; non-draft Markdown documents are shown under `/posts/`
- `public/` - Generated static site output, ignored by git

## Blog Post Format

Posts in `posts/` and archived Markdown documents in `content/` use frontmatter with fields like title, date, draft, and tags.
