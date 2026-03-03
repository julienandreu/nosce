use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use axum::{
    extract::{Path as AxumPath, Query, State},
    http::{header, StatusCode},
    middleware,
    response::{IntoResponse, Json, Response},
    routing::get,
    Router,
};
use nanospinner::{MultiSpinner, MultiSpinnerHandle};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use rust_embed::Embed;

use crate::config::ProfileDef;
use crate::fs_ops;

#[derive(Embed)]
#[folder = "static/"]
struct StaticAssets;

struct AppState {
    output_dir: PathBuf,
    base_path: String,
    profiles: Vec<ProfileDef>,
    spinner: MultiSpinnerHandle,
}

pub async fn start_server(
    output_dir: PathBuf,
    host: &str,
    port: u16,
    base_path: &str,
    profiles: Vec<ProfileDef>,
) -> Result<()> {
    let spinner = MultiSpinner::new().start();

    let state = Arc::new(AppState {
        output_dir,
        base_path: base_path.to_owned(),
        profiles,
        spinner,
    });

    let inner = Router::new()
        .route("/api/nav", get(api_nav))
        .route("/api/reports", get(api_reports))
        .route("/api/reports/{date}", get(api_report))
        .route("/api/docs/{category}", get(api_doc))
        .route(
            "/api/submodules/{name}/packages/{pkg}",
            get(api_submodule_package),
        )
        .route("/api/submodules/{name}", get(api_submodule))
        .route("/api/media/{date}", get(api_media_manifest))
        .route("/api/media/{date}/{filename}", get(api_media_file))
        .route("/api/search", get(api_search))
        .fallback(get(serve_static))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            spinner_middleware,
        ))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let app = if base_path.is_empty() {
        inner
    } else {
        Router::new().nest(base_path, inner)
    };

    let addr = format!("{host}:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("Web frontend listening on http://{addr}{base_path}");
    axum::serve(listener, app).await?;

    Ok(())
}

async fn spinner_middleware(
    State(state): State<Arc<AppState>>,
    req: axum::http::Request<axum::body::Body>,
    next: middleware::Next,
) -> Response {
    let path = req.uri().path().to_owned();

    // Only show spinners for API routes
    if !path.starts_with("/api/") {
        return next.run(req).await;
    }

    let method = req.method().clone();
    let line = state.spinner.add(format!("{method} {path}"));
    let start = Instant::now();

    let response = next.run(req).await;
    let elapsed = start.elapsed();
    let ms = elapsed.as_millis();
    let status = response.status();

    if status.is_success() {
        line.success_with(format!("{method} {path} ({ms}ms)"));
    } else {
        line.fail_with(format!("{method} {path} → {status} ({ms}ms)"));
    }

    response
}

// -- API response types --

#[derive(Serialize)]
struct ProfileInfo {
    id: String,
    label: String,
    icon: String,
    description: String,
}

impl From<&ProfileDef> for ProfileInfo {
    fn from(p: &ProfileDef) -> Self {
        Self {
            id: p.id.clone(),
            label: p.label.clone(),
            icon: p.icon.clone(),
            description: p.description.clone(),
        }
    }
}

#[derive(Serialize)]
struct SubmoduleNav {
    name: String,
    packages: Vec<String>,
}

#[derive(Serialize)]
struct NavResponse {
    latest_report: Option<String>,
    reports: Vec<String>,
    docs: Vec<String>,
    submodules: Vec<SubmoduleNav>,
    profiles: Vec<ProfileInfo>,
    media_dates: Vec<String>,
}

#[derive(Serialize)]
struct ReportEntry {
    id: String,
    label: String,
    date_range: String,
    tldr: String,
    tags: Vec<String>,
    commits: u32,
    repos: Vec<String>,
}

#[derive(Serialize)]
struct ReportsResponse {
    reports: Vec<ReportEntry>,
}

#[derive(Serialize)]
struct TocEntry {
    id: String,
    text: String,
    level: u8,
}

#[derive(Serialize)]
struct MarkdownResponse {
    html: String,
    raw: String,
    /// Which profile was served. `None` means the base (full) report.
    profile: Option<String>,
    toc: Vec<TocEntry>,
}

#[derive(Serialize)]
struct SearchResult {
    file: String,
    url: String,
    line: usize,
    context: String,
    heading: Option<String>,
}

#[derive(Serialize)]
struct SearchResponse {
    results: Vec<SearchResult>,
}

#[derive(Deserialize)]
struct SearchQuery {
    q: Option<String>,
}

#[derive(Deserialize)]
struct ReportQuery {
    profile: Option<String>,
}

#[derive(Deserialize)]
struct DocQuery {
    profile: Option<String>,
}

// -- API handlers (all fully async) --

async fn api_nav(State(state): State<Arc<AppState>>) -> Json<NavResponse> {
    let reports = fs_ops::list_report_dates(&state.output_dir).await;
    let latest_report = reports.first().cloned();

    let categories = [
        "overview",
        "architecture",
        "apis",
        "databases",
        "dependencies",
    ];
    let mut docs = Vec::new();
    for cat in &categories {
        let path = state.output_dir.join("docs").join(format!("{cat}.md"));
        if fs_ops::path_exists(&path).await {
            docs.push((*cat).to_owned());
        }
    }

    let submodule_names = fs_ops::list_submodule_names(&state.output_dir).await;
    let mut submodules = Vec::with_capacity(submodule_names.len());
    for name in submodule_names {
        let packages = fs_ops::list_submodule_packages(&state.output_dir, &name).await;
        submodules.push(SubmoduleNav { name, packages });
    }

    let profiles = state.profiles.iter().map(ProfileInfo::from).collect();
    let media_dates = fs_ops::list_media_dates(&state.output_dir).await;

    Json(NavResponse {
        latest_report,
        reports,
        docs,
        submodules,
        profiles,
        media_dates,
    })
}

async fn api_reports(State(state): State<Arc<AppState>>) -> Json<ReportsResponse> {
    let ids = fs_ops::list_report_dates(&state.output_dir).await;
    let mut reports = Vec::with_capacity(ids.len());

    for id in ids {
        let label = report_label(&id);
        let date_range = report_date_range(&id);

        // Extract metadata from the report markdown
        let path = state.output_dir.join("reports").join(format!("{id}.md"));
        let content = fs_ops::read_file(&path).await.unwrap_or_default();
        let tldr = extract_tldr(&content);
        let tags = extract_tags(&content);
        let commits = extract_commit_count(&content);
        let repos = extract_repos(&content);

        reports.push(ReportEntry {
            id,
            label,
            date_range,
            tldr,
            tags,
            commits,
            repos,
        });
    }

    Json(ReportsResponse { reports })
}

async fn api_report(
    State(state): State<Arc<AppState>>,
    AxumPath(date): AxumPath<String>,
    Query(query): Query<ReportQuery>,
) -> Result<Json<MarkdownResponse>, StatusCode> {
    // If a profile is requested, try the profile-specific report first
    if let Some(ref profile_id) = query.profile {
        let profile_path = state
            .output_dir
            .join("reports")
            .join(&date)
            .join(format!("{profile_id}.md"));

        if fs_ops::path_exists(&profile_path).await {
            let raw = fs_ops::read_file(&profile_path)
                .await
                .map_err(|_| StatusCode::NOT_FOUND)?;
            let (html, toc) = render_markdown_with_toc(&raw);
            return Ok(Json(MarkdownResponse {
                html,
                raw,
                profile: Some(profile_id.clone()),
                toc,
            }));
        }
        // Fall through to the base report if profile-specific doesn't exist
    }

    let path = state.output_dir.join("reports").join(format!("{date}.md"));
    read_and_render(&path).await
}

async fn api_doc(
    State(state): State<Arc<AppState>>,
    AxumPath(category): AxumPath<String>,
    Query(query): Query<DocQuery>,
) -> Result<Json<MarkdownResponse>, StatusCode> {
    // Try profile-specific doc first: docs/{category}/{profile}.md
    if let Some(ref profile_id) = query.profile {
        let profile_path = state
            .output_dir
            .join("docs")
            .join(&category)
            .join(format!("{profile_id}.md"));
        if fs_ops::path_exists(&profile_path).await {
            let raw = fs_ops::read_file(&profile_path)
                .await
                .map_err(|_| StatusCode::NOT_FOUND)?;
            let (html, toc) = render_markdown_with_toc(&raw);
            return Ok(Json(MarkdownResponse {
                html,
                raw,
                profile: Some(profile_id.clone()),
                toc,
            }));
        }
    }

    let path = state.output_dir.join("docs").join(format!("{category}.md"));
    read_and_render(&path).await
}

async fn api_submodule(
    State(state): State<Arc<AppState>>,
    AxumPath(name): AxumPath<String>,
    Query(query): Query<DocQuery>,
) -> Result<Json<MarkdownResponse>, StatusCode> {
    // Try profile-specific doc first: docs/submodules/{name}/{profile}.md
    if let Some(ref profile_id) = query.profile {
        let profile_path = state
            .output_dir
            .join("docs")
            .join("submodules")
            .join(&name)
            .join(format!("{profile_id}.md"));
        if fs_ops::path_exists(&profile_path).await {
            let raw = fs_ops::read_file(&profile_path)
                .await
                .map_err(|_| StatusCode::NOT_FOUND)?;
            let (html, toc) = render_markdown_with_toc(&raw);
            return Ok(Json(MarkdownResponse {
                html,
                raw,
                profile: Some(profile_id.clone()),
                toc,
            }));
        }
    }

    let path = state
        .output_dir
        .join("docs")
        .join("submodules")
        .join(format!("{name}.md"));
    read_and_render(&path).await
}

async fn api_submodule_package(
    State(state): State<Arc<AppState>>,
    AxumPath((name, pkg)): AxumPath<(String, String)>,
    Query(query): Query<DocQuery>,
) -> Result<Json<MarkdownResponse>, StatusCode> {
    // Try profile-specific package doc first
    if let Some(ref profile_id) = query.profile {
        let profile_path = state
            .output_dir
            .join("docs")
            .join("submodules")
            .join(&name)
            .join("packages")
            .join(&pkg)
            .join(format!("{profile_id}.md"));
        if fs_ops::path_exists(&profile_path).await {
            let raw = fs_ops::read_file(&profile_path)
                .await
                .map_err(|_| StatusCode::NOT_FOUND)?;
            let (html, toc) = render_markdown_with_toc(&raw);
            return Ok(Json(MarkdownResponse {
                html,
                raw,
                profile: Some(profile_id.clone()),
                toc,
            }));
        }
    }

    let path = state
        .output_dir
        .join("docs")
        .join("submodules")
        .join(&name)
        .join("packages")
        .join(format!("{pkg}.md"));
    read_and_render(&path).await
}

async fn api_search(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> Json<SearchResponse> {
    let q = query.q.unwrap_or_default();
    if q.is_empty() {
        return Json(SearchResponse {
            results: Vec::new(),
        });
    }

    let hits = fs_ops::search_all(&state.output_dir, &q, 20).await;

    let results = hits
        .into_iter()
        .map(|hit| SearchResult {
            url: relative_to_url(&hit.file),
            file: hit.file,
            line: hit.line,
            context: hit.context,
            heading: hit.heading,
        })
        .collect();

    Json(SearchResponse { results })
}

async fn api_media_manifest(
    State(state): State<Arc<AppState>>,
    AxumPath(date): AxumPath<String>,
) -> Json<serde_json::Value> {
    let manifest_path = state
        .output_dir
        .join("media")
        .join(&date)
        .join("manifest.json");
    match fs_ops::read_file(&manifest_path).await {
        Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(val) => Json(val),
            Err(_) => Json(serde_json::json!({ "date": date, "items": [] })),
        },
        Err(_) => Json(serde_json::json!({ "date": date, "items": [] })),
    }
}

async fn api_media_file(
    State(state): State<Arc<AppState>>,
    AxumPath((date, filename)): AxumPath<(String, String)>,
) -> Response {
    let media_dir = state.output_dir.join("media").join(&date);
    let candidate = media_dir.join(&filename);

    // Prevent path traversal
    match safe_resolve(&media_dir, &candidate).await {
        Some(resolved) if resolved.is_file() => match fs_ops::read_file_bytes(&resolved).await {
            Ok(bytes) => {
                let content_type = media_content_type(&resolved);
                ([(header::CONTENT_TYPE, content_type)], bytes).into_response()
            }
            Err(_) => (StatusCode::NOT_FOUND, "Not found").into_response(),
        },
        _ => (StatusCode::NOT_FOUND, "Not found").into_response(),
    }
}

async fn serve_static(
    State(state): State<Arc<AppState>>,
    req: axum::http::Request<axum::body::Body>,
) -> Response {
    let req_path = req.uri().path().trim_start_matches('/');

    // Look up the requested path in embedded assets, falling back to index.html for SPA routing
    let (data, file_path, is_index) = if req_path.is_empty() {
        match StaticAssets::get("index.html") {
            Some(asset) => (asset.data, "index.html", true),
            None => {
                return (
                    StatusCode::NOT_FOUND,
                    "Frontend not built. Run: cd webui && npm run build",
                )
                    .into_response();
            }
        }
    } else {
        match StaticAssets::get(req_path) {
            Some(asset) => (asset.data, req_path, false),
            None => {
                // SPA fallback: serve index.html for unmatched routes
                match StaticAssets::get("index.html") {
                    Some(asset) => (asset.data, "index.html", true),
                    None => {
                        return (
                            StatusCode::NOT_FOUND,
                            "Frontend not built. Run: cd webui && npm run build",
                        )
                            .into_response();
                    }
                }
            }
        }
    };

    let mime = mime_from_extension(Path::new(file_path));

    // Inject base path into index.html so the frontend knows where API lives
    if is_index && !state.base_path.is_empty() {
        let html = String::from_utf8_lossy(&data);
        let injected = html.replacen(
            "<head>",
            &format!(
                "<head><script>window.__NOSCE_BASE__=\"{}\"</script>",
                state.base_path
            ),
            1,
        );
        ([(header::CONTENT_TYPE, mime)], injected.into_bytes()).into_response()
    } else {
        ([(header::CONTENT_TYPE, mime)], data.into_owned()).into_response()
    }
}

// -- Helpers --

/// Read a markdown file and render to HTML. Non-blocking.
/// Returns the base (non-profile) response.
async fn read_and_render(path: &Path) -> Result<Json<MarkdownResponse>, StatusCode> {
    let raw = fs_ops::read_file(path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    let (html, toc) = render_markdown_with_toc(&raw);
    Ok(Json(MarkdownResponse {
        html,
        raw,
        profile: None,
        toc,
    }))
}

/// Slugify a heading text for use as an HTML id attribute.
fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Strip YAML frontmatter (--- ... ---) from the beginning of markdown content.
fn strip_frontmatter(input: &str) -> &str {
    let trimmed = input.trim_start();
    if !trimmed.starts_with("---") {
        return input;
    }
    // Find the closing ---
    let after_first = &trimmed[3..];
    if let Some(end) = after_first.find("\n---") {
        let rest = &after_first[end + 4..];
        // Skip the newline after closing ---
        rest.strip_prefix('\n').unwrap_or(rest)
    } else {
        input
    }
}

/// Render markdown to HTML using pulldown-cmark, extracting h2/h3 headings as TOC entries
/// and injecting `id` attributes into heading HTML.
fn render_markdown_with_toc(input: &str) -> (String, Vec<TocEntry>) {
    use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS;

    let content = strip_frontmatter(input);
    let parser = Parser::new_ext(content, options);

    let mut toc = Vec::new();
    let mut html = String::new();
    let mut in_heading: Option<(HeadingLevel, String)> = None;

    for event in parser {
        match event {
            Event::Start(Tag::Heading { level, .. })
                if level == HeadingLevel::H2 || level == HeadingLevel::H3 =>
            {
                // Record that we're inside a heading; write the opening tag later
                // once we know the full text (for the id attribute).
                in_heading = Some((level, String::new()));
                // Write a placeholder opening tag — we'll patch it at End
                let n = match level {
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    _ => 2,
                };
                html.push_str(&format!("<h{n}>"));
            }
            Event::Text(ref text) if in_heading.is_some() => {
                if let Some((_, ref mut buf)) = in_heading {
                    buf.push_str(text);
                }
                pulldown_cmark::html::push_html(&mut html, std::iter::once(event));
            }
            Event::Code(ref code) if in_heading.is_some() => {
                if let Some((_, ref mut buf)) = in_heading {
                    buf.push_str(code);
                }
                pulldown_cmark::html::push_html(&mut html, std::iter::once(event));
            }
            Event::End(TagEnd::Heading(level))
                if level == HeadingLevel::H2 || level == HeadingLevel::H3 =>
            {
                if let Some((hlevel, text)) = in_heading.take() {
                    let id = slugify(&text);
                    let level_num = match hlevel {
                        HeadingLevel::H2 => 2u8,
                        HeadingLevel::H3 => 3u8,
                        _ => 2u8,
                    };
                    toc.push(TocEntry {
                        id: id.clone(),
                        text,
                        level: level_num,
                    });
                    // Patch the placeholder opening tag with the id attribute
                    let tag = format!("<h{level_num}>");
                    if let Some(pos) = html.rfind(&tag) {
                        let replacement = format!("<h{level_num} id=\"{id}\">");
                        html.replace_range(pos..pos + tag.len(), &replacement);
                    }
                }
                html.push_str(&format!(
                    "</h{}>",
                    match level {
                        HeadingLevel::H2 => 2,
                        HeadingLevel::H3 => 3,
                        _ => 2,
                    }
                ));
                html.push('\n');
            }
            Event::Start(Tag::Link {
                ref dest_url,
                ref title,
                ..
            }) => {
                let url = dest_url.as_ref();
                if url.starts_with("http://") || url.starts_with("https://") {
                    html.push_str(&format!(
                        "<a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\"",
                        url
                    ));
                    if !title.is_empty() {
                        html.push_str(&format!(" title=\"{}\"", title.as_ref()));
                    }
                    html.push('>');
                } else {
                    pulldown_cmark::html::push_html(&mut html, std::iter::once(event));
                }
            }
            Event::End(TagEnd::Link) => {
                html.push_str("</a>");
            }
            _ => {
                pulldown_cmark::html::push_html(&mut html, std::iter::once(event));
            }
        }
    }

    (html, toc)
}

/// Safely resolve a candidate path ensuring it stays within the root directory.
/// Returns `None` if the path escapes the root (path traversal attempt).
async fn safe_resolve(root: &Path, candidate: &Path) -> Option<PathBuf> {
    let root = root.to_owned();
    let candidate = candidate.to_owned();
    tokio::task::spawn_blocking(move || {
        // canonicalize resolves symlinks and ../ components
        let resolved = candidate.canonicalize().ok()?;
        let root_resolved = root.canonicalize().ok()?;
        if resolved.starts_with(&root_resolved) {
            Some(resolved)
        } else {
            tracing::warn!(
                "Path traversal blocked: {} escapes {}",
                resolved.display(),
                root_resolved.display()
            );
            None
        }
    })
    .await
    .ok()
    .flatten()
}

/// Map media file extension to Content-Type header value.
fn media_content_type(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("mp4") => "video/mp4",
        Some("mov") => "video/quicktime",
        Some("webm") => "video/webm",
        _ => "application/octet-stream",
    }
}

/// Map file extension to MIME type.
fn mime_from_extension(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()) {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "application/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        Some("woff") => "font/woff",
        _ => "application/octet-stream",
    }
}

/// Produce a human-readable label for a report identifier.
/// `2026-W09` → `Week 9`, `2026-02-28` → `Feb 28`
fn report_label(id: &str) -> String {
    if let Some(rest) = id.strip_prefix("2026-W") {
        let n = rest.trim_start_matches('0');
        return format!("Week {n}");
    }
    // Daily: YYYY-MM-DD → "Mon DD" (best-effort, no chrono dep)
    if let Some(mmdd) = id.strip_prefix("2026-") {
        let parts: Vec<&str> = mmdd.split('-').collect();
        if parts.len() == 2 {
            let month = match parts[0] {
                "01" => "Jan",
                "02" => "Feb",
                "03" => "Mar",
                "04" => "Apr",
                "05" => "May",
                "06" => "Jun",
                "07" => "Jul",
                "08" => "Aug",
                "09" => "Sep",
                "10" => "Oct",
                "11" => "Nov",
                "12" => "Dec",
                _ => parts[0],
            };
            let day = parts[1].trim_start_matches('0');
            return format!("{month} {day}");
        }
    }
    id.to_owned()
}

/// Derive a date range string for a report identifier.
/// `2026-W09` → `Feb 23 – Feb 28`, `2026-02-28` → `February 28, 2026`
fn report_date_range(id: &str) -> String {
    // ISO week → date range (2026-01-01 is Thursday, ISO week 1 starts Dec 29 2025)
    if let Some(rest) = id.strip_prefix("2026-W") {
        if let Ok(week_num) = rest.parse::<u32>() {
            // ISO week 1 of 2026 starts on Mon Dec 29, 2025
            // Week N starts at Dec 29 + (N-1)*7 days
            let base_day: i32 = -3; // Dec 29 = Jan -3 (relative to Jan 1)
            let start_offset = base_day + ((week_num as i32 - 1) * 7);
            let end_offset = start_offset + 6;

            let format_day = |offset: i32| -> String {
                // Convert day-of-year offset (from Jan 1) to month + day
                let days_in_month = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
                let month_names = [
                    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov",
                    "Dec",
                ];

                if offset < 0 {
                    // December of previous year
                    return format!("Dec {}", 31 + offset + 1);
                }

                let mut remaining = offset;
                for (i, &days) in days_in_month.iter().enumerate() {
                    if remaining < days {
                        return format!("{} {}", month_names[i], remaining + 1);
                    }
                    remaining -= days;
                }
                format!("Dec {}", remaining + 1)
            };

            let start = format_day(start_offset);
            let end = format_day(end_offset);
            return format!("{start} – {end}");
        }
    }

    // Daily report
    if id.len() == 10 && id.starts_with("2026-") {
        return id.to_owned();
    }

    id.to_owned()
}

/// Extract a short TL;DR from report markdown.
/// Looks for content under `## Summary` or `## TL;DR` — handles both bullet lists and paragraphs.
fn extract_tldr(content: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();

    // Find a summary/tldr section
    let summary_start = lines.iter().position(|l| {
        let t = l.trim().to_lowercase();
        t == "## summary" || t == "## tl;dr"
    });

    if let Some(start) = summary_start {
        let mut parts: Vec<String> = Vec::new();
        for line in &lines[start + 1..] {
            let trimmed = line.trim();
            // Stop at next section or horizontal rule
            if trimmed.starts_with("## ") || trimmed.starts_with("---") {
                break;
            }
            // Skip empty lines
            if trimmed.is_empty() {
                continue;
            }
            // Bullet points
            if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
                parts.push(trimmed.replace("**", ""));
                if parts.len() >= 3 {
                    break;
                }
            } else if !trimmed.starts_with('#') && !trimmed.starts_with('>') {
                // Paragraph text — take first sentence or truncate
                let clean = trimmed.replace("**", "");
                // Truncate at ~200 chars on a word boundary
                if clean.len() > 200 {
                    let truncated = &clean[..200];
                    let end = truncated.rfind(' ').unwrap_or(200);
                    parts.push(format!("{}...", &clean[..end]));
                } else {
                    parts.push(clean);
                }
                break;
            }
        }
        if !parts.is_empty() {
            return parts.join(" ");
        }
    }

    String::new()
}

/// Extract tags from report content based on keywords and patterns.
fn extract_tags(content: &str) -> Vec<String> {
    let lower = content.to_lowercase();
    let mut tags = Vec::new();

    // Change type tags
    if lower.contains("breaking") || lower.contains("feat!") {
        tags.push("breaking".to_owned());
    }
    if lower.contains("new feature") || lower.contains("feat(") || lower.contains("feat:") {
        tags.push("feature".to_owned());
    }
    if lower.contains("bug fix") || lower.contains("fix(") || lower.contains("fix:") {
        tags.push("fix".to_owned());
    }
    if lower.contains("refactor") || lower.contains("cleanup") {
        tags.push("refactor".to_owned());
    }
    if lower.contains("security") || lower.contains("cve") || lower.contains("vulnerability") {
        tags.push("security".to_owned());
    }
    if lower.contains("performance") || lower.contains("optimize") || lower.contains("accuracy") {
        tags.push("optimization".to_owned());
    }
    if lower.contains("new customer") || lower.contains("onboard") {
        tags.push("new-customer".to_owned());
    }
    if lower.contains("release") || lower.contains("v2.") || lower.contains("v3.") {
        tags.push("release".to_owned());
    }

    tags
}

/// Extract total commit count from report content.
fn extract_commit_count(content: &str) -> u32 {
    // Look for patterns like "N new commits" or "N commits"
    for line in content.lines() {
        let lower = line.to_lowercase();
        if lower.contains("commit") {
            // Try to find a number before "commit"
            for word in lower.split_whitespace() {
                if let Ok(n) = word
                    .trim_start_matches('*')
                    .trim_start_matches('-')
                    .parse::<u32>()
                {
                    if n > 0 && n < 10000 {
                        return n;
                    }
                }
            }
        }
    }
    0
}

/// Extract which repos had changes from report content.
fn extract_repos(content: &str) -> Vec<String> {
    let mut repos = Vec::new();
    let lower = content.to_lowercase();

    if lower.contains("## desktop-app") || lower.contains("desktop-app-v2") {
        // Check it's not just "no changes"
        if !only_mentions_no_changes(&lower, "desktop-app") {
            repos.push("desktop-app".to_owned());
        }
    }
    if lower.contains("## workflow") || lower.contains("## workflows") {
        if !only_mentions_no_changes(&lower, "workflow") {
            repos.push("workflows".to_owned());
        }
    }
    if lower.contains("## sdk") {
        if !only_mentions_no_changes(&lower, "sdk") {
            repos.push("sdk".to_owned());
        }
    }

    repos
}

fn only_mentions_no_changes(content: &str, repo: &str) -> bool {
    // Crude heuristic: if the repo section only says "no changes" or "0 commits"
    if let Some(pos) = content.find(&format!("## {repo}")) {
        let section = &content[pos..];
        let section_end = section[3..].find("\n## ").map_or(section.len(), |p| p + 3);
        let section_text = &section[..section_end];
        section_text.contains("no change") || section_text.contains("0 commit")
    } else {
        true
    }
}

/// Convert a relative file path from the output dir to a frontend URL.
/// Handles profile report paths like `reports/2026-02-28/engineer.md`
/// → `/reports/2026-02-28?profile=engineer`
fn relative_to_url(relative: &str) -> String {
    let without_ext = relative.trim_end_matches(".md");
    if let Some(name) = without_ext.strip_prefix("docs/submodules/") {
        format!("/submodules/{name}")
    } else if let Some(rest) = without_ext.strip_prefix("docs/") {
        format!("/docs/{rest}")
    } else if let Some(rest) = without_ext.strip_prefix("reports/") {
        // Check for profile path: reports/YYYY-MM-DD/profile
        if let Some((date, profile)) = rest.split_once('/') {
            format!("/reports/{date}?profile={profile}")
        } else {
            format!("/reports/{rest}")
        }
    } else {
        format!("/{without_ext}")
    }
}
