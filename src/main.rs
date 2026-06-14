use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const STYLE: &str = r#"
* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}
body {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
    line-height: 1.6;
    max-width: 800px;
    margin: 0 auto;
    padding: 2rem 1rem;
    color: #333;
}
header {
    margin-bottom: 3rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid #eee;
}
header h1 a {
    text-decoration: none;
    color: #333;
    font-size: 1.8rem;
}
.post-list {
    list-style: none;
}
.post-item {
    margin-bottom: 1.5rem;
}
.post-item a {
    text-decoration: none;
    color: #0066cc;
    font-size: 1.2rem;
}
.post-item a:hover {
    text-decoration: underline;
}
.post-meta {
    color: #666;
    font-size: 0.9rem;
    margin-top: 0.3rem;
}
.post-tags {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    margin-top: 0.75rem;
}
.post-tags a {
    color: #666;
    font-size: 0.85rem;
    text-decoration: none;
}
.post-tags a:hover {
    color: #0066cc;
}
.post-content {
    margin-top: 2rem;
}
.post-content h1 {
    font-size: 2rem;
    margin-bottom: 1rem;
}
.post-content h2,
.post-content h3,
.post-content h4 {
    margin-top: 1.5rem;
    margin-bottom: 0.75rem;
}
.post-content p,
.post-content ul,
.post-content ol,
.post-content pre,
.post-content blockquote {
    margin-bottom: 1rem;
}
.post-content ul,
.post-content ol {
    padding-left: 1.5rem;
}
.post-content blockquote {
    border-left: 3px solid #ddd;
    color: #555;
    padding-left: 1rem;
}
.post-content code {
    background: #f4f4f4;
    padding: 0.2rem 0.4rem;
    border-radius: 3px;
    font-size: 0.9em;
}
.post-content pre {
    background: #f4f4f4;
    padding: 1rem;
    overflow-x: auto;
    border-radius: 5px;
}
.post-content pre code {
    background: none;
    padding: 0;
}
.post-content img {
    max-width: 100%;
    height: auto;
}
.post-content hr {
    border: 0;
    border-top: 1px solid #eee;
    margin: 1.5rem 0;
}
.back-link {
    display: inline-block;
    margin-bottom: 1.5rem;
    text-decoration: none;
    color: #666;
}
.back-link:hover {
    color: #0066cc;
}
footer {
    margin-top: 3rem;
    padding-top: 1rem;
    border-top: 1px solid #eee;
    color: #666;
    font-size: 0.9rem;
    text-align: center;
}
.empty-state {
    color: #666;
}
"#;

const HOME_STYLE: &str = r#"
* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}
html,
body {
    width: 100%;
    height: 100%;
}
body {
    overflow: hidden;
    background: #fff;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
}
.home-stage {
    position: relative;
    width: 100vw;
    height: 100vh;
    overflow: hidden;
    background: #fff;
}
.home-stage canvas {
    display: block;
    width: 100%;
    height: 100%;
    outline: none;
}
.home-entry-link {
    position: absolute;
    right: 1.25rem;
    bottom: 1.25rem;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 5rem;
    min-height: 2.5rem;
    padding: 0 1rem;
    border: 1px solid rgba(26, 28, 31, 0.22);
    border-radius: 6px;
    background: rgba(255, 255, 255, 0.72);
    color: #1a1c1f;
    font-size: 0.95rem;
    font-weight: 600;
    text-decoration: none;
    text-shadow: none;
    backdrop-filter: blur(10px);
}
.home-entry-link:hover {
    background: rgba(26, 28, 31, 0.08);
    color: #000;
}
"#;

#[derive(Clone)]
struct Config {
    base_url: String,
    language_code: String,
    title: String,
    description: String,
}

#[derive(Clone)]
struct Page {
    route: String,
    title: String,
    date_text: String,
    sort_key: String,
    draft: bool,
    tags: Vec<String>,
    html: String,
}

#[derive(Clone)]
struct TagPage {
    name: String,
    route: String,
    pages: Vec<Page>,
}

#[derive(Default)]
struct Frontmatter {
    values: BTreeMap<String, String>,
    lists: BTreeMap<String, Vec<String>>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("build");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    match command {
        "build" => build_site(&root, include_draft_posts(&args)),
        "serve" => serve_site(&root, &args),
        _ => Err(format!(
            "unknown command: {command}. Use `build` or `serve`."
        )),
    }
}

fn build_site(root: &Path, include_drafts: bool) -> Result<(), String> {
    let bevy_dist = build_home_assets(root)?;
    let config = read_config(&root.join("site.config.json"))?;
    let posts_dir = root.join("posts");
    let public_dir = root.join("public");
    let mut pages = read_site_pages(root)?;

    pages.sort_by(|a, b| b.sort_key.cmp(&a.sort_key));
    pages.retain(|page| include_drafts || !page.draft);

    let tags = collect_tags(&pages);

    if public_dir.exists() {
        fs::remove_dir_all(&public_dir).map_err(|error| format!("remove public/: {error}"))?;
    }
    fs::create_dir_all(&public_dir).map_err(|error| format!("create public/: {error}"))?;

    copy_assets(&posts_dir, &public_dir)?;
    copy_global_assets(root, &public_dir)?;
    copy_filtered(&bevy_dist, &public_dir.join("bevy"), &|_| true)?;

    for page in &pages {
        write_public_file(
            &public_dir,
            &format!("{}index.html", page.route),
            &render_article(&config, page),
        )?;
    }

    write_public_file(&public_dir, "/index.html", &render_index(&config))?;
    write_public_file(
        &public_dir,
        "/posts/index.html",
        &render_post_index(&config, &pages),
    )?;
    write_public_file(&public_dir, "/index.xml", &render_feed(&config, &pages))?;
    write_public_file(
        &public_dir,
        "/sitemap.xml",
        &render_sitemap(&config, &pages, &tags),
    )?;
    write_public_file(
        &public_dir,
        "/tags/index.html",
        &render_tag_index(&config, &tags),
    )?;

    for tag in &tags {
        write_public_file(
            &public_dir,
            &format!("{}index.html", tag.route),
            &render_tag_page(&config, tag),
        )?;
    }

    if include_drafts {
        println!(
            "Built {} page(s) into public/ including drafts.",
            pages.len()
        );
    } else {
        println!("Built {} page(s) into public/.", pages.len());
    }

    Ok(())
}

fn build_home_assets(root: &Path) -> Result<PathBuf, String> {
    let wasm_target_dir = root.join("target/wasm32-unknown-unknown/release");
    let bevy_dist = root.join("target/bevy-home");

    run_command(
        root,
        "cargo",
        &[
            "build",
            "--release",
            "--target",
            "wasm32-unknown-unknown",
            "--bin",
            "home_bevy",
        ],
    )?;

    if bevy_dist.exists() {
        fs::remove_dir_all(&bevy_dist)
            .map_err(|error| format!("remove {}: {error}", bevy_dist.display()))?;
    }
    fs::create_dir_all(&bevy_dist)
        .map_err(|error| format!("create {}: {error}", bevy_dist.display()))?;

    run_command(
        root,
        "wasm-bindgen",
        &[
            "--target",
            "web",
            "--out-dir",
            bevy_dist
                .to_str()
                .ok_or_else(|| "invalid bevy output path".to_string())?,
            "--out-name",
            "home_bevy",
            wasm_target_dir
                .join("home_bevy.wasm")
                .to_str()
                .ok_or_else(|| "invalid wasm path".to_string())?,
        ],
    )?;

    Ok(bevy_dist)
}

fn run_command(root: &Path, program: &str, args: &[&str]) -> Result<(), String> {
    let status = Command::new(program)
        .args(args)
        .current_dir(root)
        .status()
        .map_err(|error| format!("run {program}: {error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "`{} {}` failed with status {status}",
            program,
            args.join(" ")
        ))
    }
}

fn serve_site(root: &Path, args: &[String]) -> Result<(), String> {
    let include_drafts = include_draft_posts(args);
    let port = flag_value(args, "--port").unwrap_or_else(|| "1313".to_string());
    let bind = format!("127.0.0.1:{port}");

    build_site(root, include_drafts)?;
    start_polling(root.to_path_buf(), include_drafts);

    let listener = TcpListener::bind(&bind).map_err(|error| format!("bind {bind}: {error}"))?;
    println!("Serving http://{bind}/");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let root = root.to_path_buf();
                thread::spawn(move || {
                    if let Err(error) = handle_connection(&root, stream) {
                        eprintln!("request error: {error}");
                    }
                });
            }
            Err(error) => eprintln!("connection error: {error}"),
        }
    }

    Ok(())
}

fn include_draft_posts(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--draft")
}

fn flag_value(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
}

fn start_polling(root: PathBuf, include_drafts: bool) {
    thread::spawn(move || {
        let mut previous = source_signature(&root).unwrap_or_default();

        loop {
            thread::sleep(Duration::from_secs(1));
            let Ok(current) = source_signature(&root) else {
                continue;
            };

            if current != previous && build_site(&root, include_drafts).is_ok() {
                previous = current;
            }
        }
    });
}

fn handle_connection(root: &Path, mut stream: TcpStream) -> Result<(), String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .map_err(|error| error.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(30)))
        .map_err(|error| error.to_string())?;

    let mut buffer = [0; 2048];
    let size = stream
        .read(&mut buffer)
        .map_err(|error| error.to_string())?;
    if size == 0 {
        return Ok(());
    }

    let request = String::from_utf8_lossy(&buffer[..size]);
    let path = request_path(&request);
    let file_path = resolve_public_path(&root.join("public"), &path);

    match fs::read(&file_path) {
        Ok(contents) => {
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {}\r\nContent-Length: {}\r\n\r\n",
                content_type(&file_path),
                contents.len()
            );
            stream
                .write_all(header.as_bytes())
                .map_err(|error| error.to_string())?;
            stream
                .write_all(&contents)
                .map_err(|error| error.to_string())?;
        }
        Err(_) => {
            let body = b"Not found";
            let header = format!(
                "HTTP/1.1 404 Not Found\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\n\r\n",
                body.len()
            );
            stream
                .write_all(header.as_bytes())
                .map_err(|error| error.to_string())?;
            stream.write_all(body).map_err(|error| error.to_string())?;
        }
    }

    Ok(())
}

fn request_path(request: &str) -> String {
    let first_line = request.lines().next().unwrap_or_default();
    let mut parts = first_line.split_whitespace();
    let _method = parts.next();
    parts
        .next()
        .unwrap_or("/")
        .split('?')
        .next()
        .unwrap_or("/")
        .to_string()
}

fn resolve_public_path(public_dir: &Path, route: &str) -> PathBuf {
    let decoded = percent_decode(route);
    let trimmed = decoded.trim_start_matches('/');
    let mut path = PathBuf::from(public_dir);

    for part in trimmed.split('/') {
        if part == ".." || part.is_empty() {
            continue;
        }
        path.push(part);
    }

    if decoded.ends_with('/') || path.extension().is_none() {
        path.push("index.html");
    }

    path
}

fn read_config(path: &Path) -> Result<Config, String> {
    let source =
        fs::read_to_string(path).map_err(|error| format!("read site.config.json: {error}"))?;
    Ok(Config {
        base_url: json_string(&source, "baseURL")
            .unwrap_or_else(|| "https://blog.jinof.vercel.app".to_string()),
        language_code: json_string(&source, "languageCode").unwrap_or_else(|| "en-us".to_string()),
        title: json_string(&source, "title").unwrap_or_else(|| "Jinof's Blog".to_string()),
        description: json_string(&source, "description")
            .unwrap_or_else(|| "Jinof's Blog".to_string()),
    })
}

fn json_string(source: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{key}\"");
    let start = source.find(&pattern)?;
    let after_key = &source[start + pattern.len()..];
    let colon = after_key.find(':')?;
    let after_colon = after_key[colon + 1..].trim_start();
    let after_quote = after_colon.strip_prefix('"')?;
    let mut value = String::new();
    let mut escaped = false;

    for ch in after_quote.chars() {
        if escaped {
            value.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == '"' {
            return Some(value);
        }
        value.push(ch);
    }

    None
}

fn read_site_pages(root: &Path) -> Result<Vec<Page>, String> {
    let mut by_route = BTreeMap::new();

    for page in read_pages(&root.join("content"), true)? {
        by_route.insert(page.route.clone(), page);
    }
    for page in read_pages(&root.join("posts"), false)? {
        by_route.insert(page.route.clone(), page);
    }

    Ok(by_route.into_values().collect())
}

fn read_pages(content_dir: &Path, strip_archive_posts_prefix: bool) -> Result<Vec<Page>, String> {
    if !content_dir.exists() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    collect_markdown_files(content_dir, &mut files)?;
    files
        .into_iter()
        .map(|file| read_page(content_dir, &file, strip_archive_posts_prefix))
        .collect()
}

fn collect_markdown_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(dir).map_err(|error| format!("read {}: {error}", dir.display()))? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_markdown_files(&path, files)?;
        } else if is_markdown_document(&path) {
            files.push(path);
        }
    }
    Ok(())
}

fn is_markdown_document(path: &Path) -> bool {
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with('.'))
    {
        return false;
    }

    match path.extension().and_then(|ext| ext.to_str()) {
        Some("md" | "markdown") => true,
        Some(_) => false,
        None => true,
    }
}

fn read_page(
    content_dir: &Path,
    file: &Path,
    strip_archive_posts_prefix: bool,
) -> Result<Page, String> {
    let source =
        fs::read_to_string(file).map_err(|error| format!("read {}: {error}", file.display()))?;
    let (frontmatter, body) = parse_frontmatter(&source);
    let title = frontmatter
        .values
        .get("title")
        .cloned()
        .unwrap_or_else(|| title_from_file(file));
    let date = frontmatter.values.get("date").cloned().unwrap_or_default();
    let date_text = date.get(0..10).unwrap_or("").to_string();
    let sort_key = if date.is_empty() {
        "0000-00-00".to_string()
    } else {
        date.clone()
    };
    let tags = frontmatter.lists.get("tags").cloned().unwrap_or_default();

    Ok(Page {
        route: route_for(content_dir, file, strip_archive_posts_prefix)?,
        title,
        date_text,
        sort_key,
        draft: frontmatter
            .values
            .get("draft")
            .map(|value| value == "true")
            .unwrap_or(false),
        tags,
        html: markdown_to_html(body.trim()),
    })
}

fn parse_frontmatter(source: &str) -> (Frontmatter, String) {
    let normalized = source.replace("\r\n", "\n");
    let mut lines: Vec<&str> = normalized.lines().collect();

    if lines.first().copied() != Some("---") {
        return (Frontmatter::default(), normalized);
    }

    let Some(end) = lines
        .iter()
        .skip(1)
        .position(|line| *line == "---")
        .map(|index| index + 1)
    else {
        return (Frontmatter::default(), normalized);
    };

    let mut frontmatter = Frontmatter::default();
    let mut current_key = String::new();

    for line in &lines[1..end] {
        let trimmed = line.trim();
        if let Some(item) = trimmed.strip_prefix("- ") {
            if !current_key.is_empty() {
                frontmatter
                    .lists
                    .entry(current_key.clone())
                    .or_default()
                    .push(parse_scalar(item));
            }
            continue;
        }

        if let Some((key, value)) = trimmed.split_once(':') {
            current_key = key.trim().to_string();
            let value = value.trim();
            if value.is_empty() {
                frontmatter.lists.entry(current_key.clone()).or_default();
            } else {
                frontmatter
                    .values
                    .insert(current_key.clone(), parse_scalar(value));
            }
        }
    }

    let body = lines.split_off(end + 1).join("\n");
    (frontmatter, body)
}

fn parse_scalar(value: &str) -> String {
    let trimmed = value.trim();
    if (trimmed.starts_with('"') && trimmed.ends_with('"'))
        || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
    {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

fn route_for(
    content_dir: &Path,
    file: &Path,
    strip_archive_posts_prefix: bool,
) -> Result<String, String> {
    let relative = file
        .strip_prefix(content_dir)
        .map_err(|error| error.to_string())?;
    let mut parts = Vec::new();

    if let Some(parent) = relative.parent() {
        for part in parent.components() {
            let slug = slugify(&part.as_os_str().to_string_lossy());
            if strip_archive_posts_prefix && parts.is_empty() && slug == "posts" {
                continue;
            }
            parts.push(slug);
        }
    }

    let stem = file
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("post");
    parts.push(slugify(stem));
    Ok(format!("/posts/{}/", parts.join("/")))
}

fn title_from_file(file: &Path) -> String {
    file.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("Post")
        .replace(['-', '_'], " ")
}

fn slugify(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .replace(['"', '\''], "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}

fn collect_tags(pages: &[Page]) -> Vec<TagPage> {
    let mut names = BTreeSet::new();
    for page in pages {
        for tag in &page.tags {
            names.insert(tag.clone());
        }
    }

    names
        .into_iter()
        .map(|name| TagPage {
            route: format!("/tags/{}/", slugify(&name)),
            pages: pages
                .iter()
                .filter(|page| page.tags.contains(&name))
                .cloned()
                .collect(),
            name,
        })
        .collect()
}

fn copy_assets(content_dir: &Path, public_dir: &Path) -> Result<(), String> {
    if !content_dir.exists() {
        return Ok(());
    }

    copy_filtered(content_dir, public_dir, &|path| {
        matches!(
            path.extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("")
                .to_lowercase()
                .as_str(),
            "apng"
                | "avif"
                | "css"
                | "gif"
                | "ico"
                | "jpeg"
                | "jpg"
                | "js"
                | "pdf"
                | "png"
                | "svg"
                | "txt"
                | "webp"
                | "xml"
        )
    })
}

fn copy_global_assets(root: &Path, public_dir: &Path) -> Result<(), String> {
    let assets_dir = root.join("assets");
    if assets_dir.exists() {
        copy_filtered(&assets_dir, public_dir, &|path| {
            path.file_name().and_then(|name| name.to_str()) != Some(".gitkeep")
        })?;
    }
    Ok(())
}

fn copy_filtered(
    source: &Path,
    destination: &Path,
    should_copy: &dyn Fn(&Path) -> bool,
) -> Result<(), String> {
    for entry in
        fs::read_dir(source).map_err(|error| format!("read {}: {error}", source.display()))?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        let from = entry.path();
        let to = destination.join(entry.file_name());
        if from.is_dir() {
            fs::create_dir_all(&to).map_err(|error| format!("create {}: {error}", to.display()))?;
            copy_filtered(&from, &to, should_copy)?;
        } else if should_copy(&from) {
            if let Some(parent) = to.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("create {}: {error}", parent.display()))?;
            }
            fs::copy(&from, &to).map_err(|error| format!("copy {}: {error}", from.display()))?;
        }
    }
    Ok(())
}

fn write_public_file(public_dir: &Path, route: &str, contents: &str) -> Result<(), String> {
    let target = public_dir.join(route.trim_start_matches('/'));
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create {}: {error}", parent.display()))?;
    }
    fs::write(&target, contents).map_err(|error| format!("write {}: {error}", target.display()))
}

fn render_shell(config: &Config, title: &str, canonical_path: &str, main: &str) -> String {
    let page_title = if title == config.title {
        config.title.clone()
    } else {
        format!("{title} | {}", config.title)
    };
    let canonical = absolute_url(config, canonical_path);

    format!(
        r#"<!DOCTYPE html>
<html lang="{language}">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{page_title}</title>
    <link rel="canonical" href="{canonical}">
    <style>
{style}
    </style>
</head>
<body>
    <header>
        <h1><a href="/">{site_title}</a></h1>
    </header>

    <main>
{main}
    </main>

    <footer>
        &copy; {year} {site_title}. All rights reserved.
    </footer>
</body>
</html>
"#,
        language = escape_attr(&config.language_code),
        page_title = escape_html(&page_title),
        canonical = escape_attr(&canonical),
        style = STYLE.trim(),
        site_title = escape_html(&config.title),
        main = main,
        year = current_year()
    )
}

fn render_index(config: &Config) -> String {
    let canonical = absolute_url(config, "/");

    format!(
        r#"<!DOCTYPE html>
<html lang="{language}">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{page_title}</title>
    <link rel="canonical" href="{canonical}">
    <style>
{style}
    </style>
</head>
<body class="home-page">
<main class="home-stage" aria-label="Interactive Bevy homepage">
    <canvas id="bevy-home-canvas"></canvas>
    <a class="home-entry-link" href="/posts/">Posts</a>
</main>
<script type="module">
try {{
    const bevy = await import("/bevy/home_bevy.js");
    await bevy.default();
}} catch (error) {{
    console.error("Failed to load Bevy homepage", error);
}}
</script>
</body>
</html>
"#,
        language = escape_attr(&config.language_code),
        page_title = escape_html(&config.title),
        canonical = escape_attr(&canonical),
        style = HOME_STYLE.trim()
    )
}

fn render_post_index(config: &Config, pages: &[Page]) -> String {
    let main = if pages.is_empty() {
        "<p class=\"empty-state\">No posts yet.</p>".to_string()
    } else {
        let items = pages
            .iter()
            .map(|page| {
                format!(
                    r#"    <li class="post-item">
        <a href="{href}">{title}</a>
        <div class="post-meta">{date}</div>
    </li>"#,
                    href = escape_attr(&encode_uri(&page.route)),
                    title = escape_html(&page.title),
                    date = escape_html(&page.date_text)
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        format!("<ul class=\"post-list\">\n{items}\n</ul>")
    };

    render_shell(config, "Posts", "/posts/", &main)
}

fn render_article(config: &Config, page: &Page) -> String {
    let tag_links = if page.tags.is_empty() {
        String::new()
    } else {
        let links = page
            .tags
            .iter()
            .map(|tag| {
                let route = format!("/tags/{}/", slugify(tag));
                format!(
                    r##"<a href="{}">#{}</a>"##,
                    escape_attr(&encode_uri(&route)),
                    escape_html(tag)
                )
            })
            .collect::<Vec<_>>()
            .join("");
        format!("<div class=\"post-tags\">{links}</div>")
    };

    let main = format!(
        r#"<article class="post-content">
    <a href="/posts/" class="back-link">&larr; Back to posts</a>
    <h1>{title}</h1>
    <div class="post-meta">{date}</div>
    {tag_links}
{body}
</article>"#,
        title = escape_html(&page.title),
        date = escape_html(&page.date_text),
        tag_links = tag_links,
        body = page.html
    );

    render_shell(config, &page.title, &page.route, &main)
}

fn render_tag_index(config: &Config, tags: &[TagPage]) -> String {
    let items = tags
        .iter()
        .map(|tag| {
            format!(
                r#"    <li class="post-item">
        <a href="{href}">{name}</a>
        <div class="post-meta">{count} post(s)</div>
    </li>"#,
                href = escape_attr(&encode_uri(&tag.route)),
                name = escape_html(&tag.name),
                count = tag.pages.len()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    render_shell(
        config,
        "Tags",
        "/tags/",
        &format!("<ul class=\"post-list\">\n{items}\n</ul>"),
    )
}

fn render_tag_page(config: &Config, tag: &TagPage) -> String {
    let items = tag
        .pages
        .iter()
        .map(|page| {
            format!(
                r#"    <li class="post-item">
        <a href="{href}">{title}</a>
        <div class="post-meta">{date}</div>
    </li>"#,
                href = escape_attr(&encode_uri(&page.route)),
                title = escape_html(&page.title),
                date = escape_html(&page.date_text)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    render_shell(
        config,
        &format!("Tag: {}", tag.name),
        &tag.route,
        &format!("<ul class=\"post-list\">\n{items}\n</ul>"),
    )
}

fn render_feed(config: &Config, pages: &[Page]) -> String {
    let items = pages
        .iter()
        .take(20)
        .map(|page| {
            let url = absolute_url(config, &page.route);
            format!(
                r#"<item>
    <title>{title}</title>
    <link>{url}</link>
    <guid>{url}</guid>
    <pubDate>{date}</pubDate>
    <description>{description}</description>
</item>"#,
                title = escape_xml(&page.title),
                url = escape_xml(&url),
                date = escape_xml(&page.date_text),
                description =
                    escape_xml(&strip_html(&page.html).chars().take(500).collect::<String>())
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
<channel>
    <title>{title}</title>
    <link>{base_url}</link>
    <description>{description}</description>
{items}
</channel>
</rss>
"#,
        title = escape_xml(&config.title),
        base_url = escape_xml(&config.base_url),
        description = escape_xml(&config.description),
        items = items
    )
}

fn render_sitemap(config: &Config, pages: &[Page], tags: &[TagPage]) -> String {
    let mut routes = vec!["/".to_string(), "/posts/".to_string()];
    routes.extend(pages.iter().map(|page| page.route.clone()));
    routes.push("/tags/".to_string());
    routes.extend(tags.iter().map(|tag| tag.route.clone()));

    let urls = routes
        .iter()
        .map(|route| {
            format!(
                "  <url>\n    <loc>{}</loc>\n  </url>",
                escape_xml(&absolute_url(config, route))
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
{urls}
</urlset>
"#
    )
}

fn markdown_to_html(markdown: &str) -> String {
    let lines: Vec<&str> = markdown.lines().collect();
    let mut html = Vec::new();
    let mut paragraph: Vec<String> = Vec::new();
    let mut list_type: Option<&str> = None;
    let mut list_items: Vec<String> = Vec::new();
    let mut quote: Vec<String> = Vec::new();
    let mut fence: Option<(String, String)> = None;
    let mut code_lines: Vec<String> = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        let line = lines[index];
        let trimmed = line.trim();

        if let Some((marker, language)) = fence.clone() {
            if trimmed.starts_with(&marker) {
                html.push(render_code_block(&code_lines, &language));
                fence = None;
                code_lines.clear();
            } else {
                code_lines.push(line.to_string());
            }
            index += 1;
            continue;
        }

        if trimmed.is_empty() {
            flush_paragraph(&mut html, &mut paragraph);
            flush_list(&mut html, &mut list_type, &mut list_items);
            flush_quote(&mut html, &mut quote);
            index += 1;
            continue;
        }

        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            flush_paragraph(&mut html, &mut paragraph);
            flush_list(&mut html, &mut list_type, &mut list_items);
            flush_quote(&mut html, &mut quote);
            let marker = trimmed.chars().take(3).collect::<String>();
            let language = trimmed[3..].trim().to_string();
            fence = Some((marker, language));
            index += 1;
            continue;
        }

        if trimmed == "***" || trimmed.chars().all(|ch| ch == '-') && trimmed.len() >= 3 {
            flush_paragraph(&mut html, &mut paragraph);
            flush_list(&mut html, &mut list_type, &mut list_items);
            flush_quote(&mut html, &mut quote);
            html.push("<hr>".to_string());
            index += 1;
            continue;
        }

        if let Some(rest) = line.strip_prefix("> ") {
            flush_paragraph(&mut html, &mut paragraph);
            flush_list(&mut html, &mut list_type, &mut list_items);
            quote.push(rest.to_string());
            index += 1;
            continue;
        }
        if line == ">" {
            quote.push(String::new());
            index += 1;
            continue;
        }
        flush_quote(&mut html, &mut quote);

        if let Some((level, text)) = parse_heading(line) {
            flush_paragraph(&mut html, &mut paragraph);
            flush_list(&mut html, &mut list_type, &mut list_items);
            let id = heading_id(text);
            html.push(format!(
                "<h{level} id=\"{}\">{}</h{level}>",
                escape_attr(&id),
                render_inline(text)
            ));
            index += 1;
            continue;
        }

        if let Some(item) = unordered_item(line) {
            flush_paragraph(&mut html, &mut paragraph);
            push_list_item("ul", item, &mut html, &mut list_type, &mut list_items);
            index += 1;
            continue;
        }

        if let Some(item) = ordered_item(line) {
            flush_paragraph(&mut html, &mut paragraph);
            push_list_item("ol", item, &mut html, &mut list_type, &mut list_items);
            index += 1;
            continue;
        }

        if line.starts_with("    ") {
            flush_paragraph(&mut html, &mut paragraph);
            flush_list(&mut html, &mut list_type, &mut list_items);
            flush_quote(&mut html, &mut quote);
            let mut code = vec![line.trim_start_matches("    ").to_string()];
            while index + 1 < lines.len() && lines[index + 1].starts_with("    ") {
                index += 1;
                code.push(lines[index].trim_start_matches("    ").to_string());
            }
            html.push(render_code_block(&code, ""));
            index += 1;
            continue;
        }

        flush_list(&mut html, &mut list_type, &mut list_items);
        paragraph.push(trimmed.to_string());
        index += 1;
    }

    flush_paragraph(&mut html, &mut paragraph);
    flush_list(&mut html, &mut list_type, &mut list_items);
    flush_quote(&mut html, &mut quote);
    if fence.is_some() {
        html.push(render_code_block(&code_lines, ""));
    }

    html.join("\n")
}

fn flush_paragraph(html: &mut Vec<String>, paragraph: &mut Vec<String>) {
    if paragraph.is_empty() {
        return;
    }
    html.push(format!("<p>{}</p>", render_inline(&paragraph.join(" "))));
    paragraph.clear();
}

fn flush_list(html: &mut Vec<String>, list_type: &mut Option<&str>, list_items: &mut Vec<String>) {
    let Some(kind) = list_type.take() else {
        return;
    };
    let items = list_items
        .iter()
        .map(|item| format!("  <li>{}</li>", render_inline(item)))
        .collect::<Vec<_>>()
        .join("\n");
    html.push(format!("<{kind}>\n{items}\n</{kind}>"));
    list_items.clear();
}

fn flush_quote(html: &mut Vec<String>, quote: &mut Vec<String>) {
    if quote.is_empty() {
        return;
    }
    html.push(format!(
        "<blockquote>\n{}\n</blockquote>",
        markdown_to_html(&quote.join("\n"))
    ));
    quote.clear();
}

fn push_list_item<'a>(
    kind: &'a str,
    item: &str,
    html: &mut Vec<String>,
    list_type: &mut Option<&'a str>,
    list_items: &mut Vec<String>,
) {
    if list_type.is_some_and(|current| current != kind) {
        flush_list(html, list_type, list_items);
    }
    if list_type.is_none() {
        *list_type = Some(kind);
    }
    list_items.push(item.trim().to_string());
}

fn parse_heading(line: &str) -> Option<(usize, &str)> {
    let hashes = line.chars().take_while(|ch| *ch == '#').count();
    if hashes == 0 || hashes > 6 || !line.chars().nth(hashes).is_some_and(|ch| ch == ' ') {
        return None;
    }
    Some((hashes, line[hashes + 1..].trim()))
}

fn unordered_item(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    trimmed
        .strip_prefix("- ")
        .or_else(|| trimmed.strip_prefix("* "))
}

fn ordered_item(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    let dot = trimmed.find('.')?;
    if dot == 0 || !trimmed[..dot].chars().all(|ch| ch.is_ascii_digit()) {
        return None;
    }
    trimmed[dot + 1..].strip_prefix(' ')
}

fn render_code_block(lines: &[String], language: &str) -> String {
    let class = if language.is_empty() {
        String::new()
    } else {
        format!(" class=\"language-{}\"", escape_attr(language))
    };
    format!(
        "<pre><code{class}>{}</code></pre>",
        escape_html(&lines.join("\n"))
    )
}

fn render_inline(markdown: &str) -> String {
    let mut output = String::new();
    let mut rest = markdown;

    while let Some(start) = rest.find('`') {
        output.push_str(&render_links_and_images(&rest[..start]));
        let after = &rest[start + 1..];
        if let Some(end) = after.find('`') {
            output.push_str("<code>");
            output.push_str(&escape_html(&after[..end]));
            output.push_str("</code>");
            rest = &after[end + 1..];
        } else {
            output.push_str(&escape_html(&rest[start..]));
            rest = "";
        }
    }

    output.push_str(&render_links_and_images(rest));
    output
}

fn render_links_and_images(text: &str) -> String {
    let mut output = String::new();
    let mut rest = text;

    while let Some(open) = rest.find('[') {
        output.push_str(&escape_html(&rest[..open]));
        let image = open > 0 && rest.as_bytes()[open - 1] == b'!';
        if image && output.ends_with('!') {
            output.pop();
        }

        let Some(close) = rest[open + 1..].find(']').map(|index| index + open + 1) else {
            output.push_str(&escape_html(&rest[open..]));
            return output;
        };
        let after = &rest[close + 1..];
        if !after.starts_with('(') {
            output.push_str(&escape_html(&rest[open..close + 1]));
            rest = after;
            continue;
        }
        let Some(url_end) = after[1..].find(')').map(|index| index + 1) else {
            output.push_str(&escape_html(&rest[open..]));
            return output;
        };

        let label = &rest[open + 1..close];
        let url = normalize_url(&after[1..url_end]);
        if image {
            output.push_str(&format!(
                r#"<img src="{}" alt="{}">"#,
                escape_attr(&url),
                escape_attr(label)
            ));
        } else {
            output.push_str(&format!(
                r#"<a href="{}">{}</a>"#,
                escape_attr(&url),
                escape_html(label)
            ));
        }
        rest = &after[url_end + 1..];
    }

    output.push_str(&escape_html(rest));
    output
}

fn normalize_url(url: &str) -> String {
    let trimmed = url.trim().trim_matches('"').trim_matches('\'');
    if trimmed.starts_with("http:")
        || trimmed.starts_with("https:")
        || trimmed.starts_with("mailto:")
        || trimmed.starts_with('#')
        || trimmed.starts_with('/')
    {
        trimmed.to_string()
    } else {
        format!("/{}", trimmed.trim_start_matches("./"))
    }
}

fn heading_id(markdown: &str) -> String {
    let mut text = String::new();
    let mut in_link = false;
    for ch in markdown.to_lowercase().chars() {
        match ch {
            '[' => in_link = true,
            ']' => in_link = false,
            '(' => break,
            _ if !in_link
                && (ch.is_alphanumeric()
                    || ch.is_whitespace()
                    || ch == '-'
                    || ch == '_'
                    || ch == '.') =>
            {
                text.push(ch)
            }
            _ if in_link => text.push(ch),
            _ => {}
        }
    }
    let id = text.split_whitespace().collect::<Vec<_>>().join("-");
    if id.is_empty() {
        "section".to_string()
    } else {
        id
    }
}

fn source_signature(root: &Path) -> Result<String, String> {
    let mut files = Vec::new();
    collect_all_files(&root.join("content"), &mut files)?;
    collect_all_files(&root.join("posts"), &mut files)?;
    collect_all_files(&root.join("assets"), &mut files)?;
    collect_all_files(&root.join("src"), &mut files)?;
    files.push(root.join("site.config.json"));
    files.push(root.join("Cargo.toml"));

    let mut parts = Vec::new();
    for file in files {
        let metadata =
            fs::metadata(&file).map_err(|error| format!("stat {}: {error}", file.display()))?;
        let modified = metadata
            .modified()
            .unwrap_or(SystemTime::UNIX_EPOCH)
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let relative = file.strip_prefix(root).unwrap_or(&file).display();
        parts.push(format!("{relative}:{modified}:{}", metadata.len()));
    }
    parts.sort();
    Ok(parts.join("\n"))
}

fn collect_all_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if !dir.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(dir).map_err(|error| format!("read {}: {error}", dir.display()))? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            collect_all_files(&path, files)?;
        } else {
            files.push(path);
        }
    }
    Ok(())
}

fn content_type(file: &Path) -> &'static str {
    match file.extension().and_then(|ext| ext.to_str()).unwrap_or("") {
        "css" => "text/css; charset=utf-8",
        "gif" => "image/gif",
        "html" => "text/html; charset=utf-8",
        "ico" => "image/x-icon",
        "jpg" | "jpeg" => "image/jpeg",
        "js" => "text/javascript; charset=utf-8",
        "png" => "image/png",
        "svg" => "image/svg+xml",
        "txt" => "text/plain; charset=utf-8",
        "wasm" => "application/wasm",
        "webp" => "image/webp",
        "xml" => "application/xml; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn absolute_url(config: &Config, route: &str) -> String {
    let base = config.base_url.trim_end_matches('/');
    let path = route.trim_start_matches('/');
    if path.is_empty() {
        format!("{base}/")
    } else {
        format!("{base}/{}", encode_uri(path))
    }
}

fn encode_uri(value: &str) -> String {
    let mut output = String::new();
    for byte in value.as_bytes() {
        let keep =
            byte.is_ascii_alphanumeric() || matches!(*byte, b'-' | b'_' | b'.' | b'~' | b'/');
        if keep {
            output.push(*byte as char);
        } else {
            output.push_str(&format!("%{byte:02X}"));
        }
    }
    output
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut output = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(hex) = u8::from_str_radix(&value[index + 1..index + 3], 16) {
                output.push(hex);
                index += 3;
                continue;
            }
        }
        output.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&output).to_string()
}

fn strip_html(html: &str) -> String {
    let mut output = String::new();
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => {
                in_tag = false;
                output.push(' ');
            }
            _ if !in_tag => output.push(ch),
            _ => {}
        }
    }
    output.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn current_year() -> i32 {
    let days = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        / 86_400;
    civil_year_from_days(days as i64)
}

fn civil_year_from_days(days_since_unix_epoch: i64) -> i32 {
    let z = days_since_unix_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let month = mp + if mp < 10 { 3 } else { -9 };
    (y + if month <= 2 { 1 } else { 0 }) as i32
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn escape_attr(value: &str) -> String {
    escape_html(value).replace('\'', "&#39;")
}

fn escape_xml(value: &str) -> String {
    escape_html(value).replace('\'', "&apos;")
}
