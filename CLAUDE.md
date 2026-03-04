# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a personal Hugo blog deployed to Vercel (https://blog.jinof.vercel.app). The repository contains blog posts written in Markdown.

## Commands

- **Build the site**: `hugo` - Generates static site files in `public/`
- **Serve locally**: `hugo server` - Runs a local development server with live reload
- **Deploy**: `./delpoy.sh "commit message"` - Builds the site and pushes to the public folder in git (the public folder is a submodule that deploys to Vercel)

## Code Structure

- `hugo.toml` - Main Hugo configuration (baseURL, language, title)
- `content/posts/` - Blog posts in Markdown format
- `layouts/` - Custom Hugo templates
- `static/` - Static assets (images, CSS, JS)
- `public/` - Generated static site (git submodule for Vercel deployment)

## Blog Post Format

Posts in `content/posts/` use standard Hugo frontmatter with fields like title, date, and tags.
