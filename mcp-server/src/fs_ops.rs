//! Non-blocking filesystem operations shared between MCP server and web frontend.
//!
//! All public functions are async and use `tokio::task::spawn_blocking` to avoid
//! blocking the tokio runtime. Directory traversals with `walkdir` are inherently
//! synchronous, so they run on the blocking threadpool.

use std::path::{Path, PathBuf};

use tokio::task;

/// Read a file to string, non-blocking.
pub async fn read_file(path: &Path) -> std::io::Result<String> {
    let path = path.to_owned();
    task::spawn_blocking(move || std::fs::read_to_string(&path))
        .await
        .expect("spawn_blocking panicked")
}

/// Read a file to bytes, non-blocking.
pub async fn read_file_bytes(path: &Path) -> std::io::Result<Vec<u8>> {
    let path = path.to_owned();
    task::spawn_blocking(move || std::fs::read(&path))
        .await
        .expect("spawn_blocking panicked")
}

/// Check if a path exists, non-blocking.
pub async fn path_exists(path: &Path) -> bool {
    let path = path.to_owned();
    task::spawn_blocking(move || path.exists())
        .await
        .unwrap_or(false)
}

/// List report dates (YYYY-MM-DD) from the reports/ directory, most recent first.
pub async fn list_report_dates(output_dir: &Path) -> Vec<String> {
    let reports_dir = output_dir.join("reports");
    task::spawn_blocking(move || list_report_dates_sync(&reports_dir))
        .await
        .unwrap_or_default()
}

/// Find the path to the latest report file.
pub async fn find_latest_report(output_dir: &Path) -> Option<PathBuf> {
    let reports_dir = output_dir.join("reports");
    task::spawn_blocking(move || {
        let mut files: Vec<PathBuf> = list_md_files_sync(&reports_dir);
        files.sort_unstable();
        files.pop()
    })
    .await
    .ok()
    .flatten()
}

/// List submodule doc names (without .md extension) from docs/submodules/.
pub async fn list_submodule_names(output_dir: &Path) -> Vec<String> {
    let dir = output_dir.join("docs").join("submodules");
    task::spawn_blocking(move || list_md_stems_sync(&dir))
        .await
        .unwrap_or_default()
}

/// List package doc names for a submodule from docs/submodules/{name}/packages/.
pub async fn list_submodule_packages(output_dir: &Path, submodule: &str) -> Vec<String> {
    let dir = output_dir
        .join("docs")
        .join("submodules")
        .join(submodule)
        .join("packages");
    task::spawn_blocking(move || list_md_stems_sync(&dir))
        .await
        .unwrap_or_default()
}

/// List dates that have media manifests in the media/ directory.
/// Scans `<output>/media/` for subdirectories containing `manifest.json`.
pub async fn list_media_dates(output_dir: &Path) -> Vec<String> {
    let media_dir = output_dir.join("media");
    task::spawn_blocking(move || list_media_dates_sync(&media_dir))
        .await
        .unwrap_or_default()
}

/// A single search hit across docs/reports.
pub struct SearchHit {
    pub file: String,
    pub line: usize,
    pub context: String,
    /// The nearest `## ` heading above the match, if any.
    pub heading: Option<String>,
}

/// Full-text search across all markdown files in the output directory.
/// Runs on the blocking threadpool since walkdir + fs reads are synchronous.
pub async fn search_all(output_dir: &Path, query: &str, limit: usize) -> Vec<SearchHit> {
    let output_dir = output_dir.to_owned();
    let query = query.to_lowercase();
    task::spawn_blocking(move || search_sync(&output_dir, &query, limit))
        .await
        .unwrap_or_default()
}

/// Extract a submodule's section from a daily report markdown.
/// Looks for `## <name>` heading and captures until the next `## ` or end of file.
pub fn extract_submodule_section(content: &str, name: &str) -> Option<String> {
    let header = format!("## {name}");
    let lines: Vec<&str> = content.lines().collect();

    let start = lines
        .iter()
        .position(|line| line.trim().eq_ignore_ascii_case(&header))?;

    let end = lines
        .iter()
        .skip(start + 1)
        .position(|line| line.starts_with("## "))
        .map_or(lines.len(), |pos| start + 1 + pos);

    let section = lines[start + 1..end].join("\n");
    let trimmed = section.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_owned())
    }
}

// --- Synchronous helpers (run inside spawn_blocking) ---

fn list_report_dates_sync(reports_dir: &Path) -> Vec<String> {
    let mut dates: Vec<String> = list_md_stems_sync(reports_dir);
    dates.sort_unstable();
    dates.reverse();
    dates
}

fn list_md_files_sync(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(err) => {
            if err.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!("Failed to read directory {}: {}", dir.display(), err);
            }
            return files;
        }
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.ends_with(".md") {
            files.push(entry.path());
        }
    }
    files
}

fn list_md_stems_sync(dir: &Path) -> Vec<String> {
    let mut names = Vec::new();
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(err) => {
            if err.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!("Failed to read directory {}: {}", dir.display(), err);
            }
            return names;
        }
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.ends_with(".md") {
            names.push(name_str.trim_end_matches(".md").to_owned());
        }
    }
    names.sort_unstable();
    names
}

fn list_media_dates_sync(media_dir: &Path) -> Vec<String> {
    let mut dates = Vec::new();
    let entries = match std::fs::read_dir(media_dir) {
        Ok(e) => e,
        Err(err) => {
            if err.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(
                    "Failed to read media directory {}: {}",
                    media_dir.display(),
                    err
                );
            }
            return dates;
        }
    };
    for entry in entries.flatten() {
        if entry.file_type().map_or(false, |ft| ft.is_dir()) {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            let manifest = entry.path().join("manifest.json");
            if manifest.is_file() {
                dates.push(name_str.into_owned());
            }
        }
    }
    dates.sort_unstable();
    dates.reverse();
    dates
}

fn search_sync(output_dir: &Path, query: &str, limit: usize) -> Vec<SearchHit> {
    let mut results = Vec::new();

    for subdir in &["docs", "reports"] {
        let dir = output_dir.join(subdir);
        if !dir.is_dir() {
            continue;
        }

        let walker = walkdir::WalkDir::new(&dir).max_depth(3);
        for entry in walker.into_iter().flatten() {
            if results.len() >= limit {
                return results;
            }
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }

            let content = match std::fs::read_to_string(path) {
                Ok(c) => c,
                Err(err) => {
                    tracing::warn!("Failed to read {}: {}", path.display(), err);
                    continue;
                }
            };

            let relative = path
                .strip_prefix(output_dir)
                .unwrap_or(path)
                .display()
                .to_string();

            // Collect lines once per file, not per match
            let lines: Vec<&str> = content.lines().collect();

            for (i, line) in lines.iter().enumerate() {
                if results.len() >= limit {
                    break;
                }
                if line.to_lowercase().contains(query) {
                    let start = i.saturating_sub(1);
                    let end = (i + 2).min(lines.len());
                    let snippet = lines[start..end].join("\n");

                    // Find nearest heading above this line
                    let heading = lines[..i]
                        .iter()
                        .rev()
                        .find(|l| l.starts_with("## "))
                        .map(|l| l.trim_start_matches("## ").to_owned());

                    results.push(SearchHit {
                        file: relative.clone(),
                        line: i + 1,
                        context: snippet,
                        heading,
                    });
                }
            }
        }
    }

    results
}
