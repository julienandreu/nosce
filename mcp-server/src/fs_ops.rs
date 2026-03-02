//! Non-blocking filesystem operations shared between MCP server and web frontend.
//!
//! All public functions are async and use `tokio::task::spawn_blocking` to avoid
//! blocking the tokio runtime. Directory traversals with `walkdir` are inherently
//! synchronous, so they run on the blocking threadpool.
//!
//! Write operations include path traversal guards to ensure all writes stay
//! within the output directory.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
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

// --- Submodule discovery ---

/// A git submodule discovered from `.gitmodules`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmoduleInfo {
    pub name: String,
    pub path: String,
    pub url: String,
    pub branch: String,
}

/// Parse `.gitmodules` in the given input directory and return all submodules.
pub async fn discover_submodules(input_dir: &Path) -> Result<Vec<SubmoduleInfo>> {
    let input_dir = input_dir.to_owned();
    task::spawn_blocking(move || discover_submodules_sync(&input_dir))
        .await
        .context("spawn_blocking panicked")?
}

fn discover_submodules_sync(input_dir: &Path) -> Result<Vec<SubmoduleInfo>> {
    let gitmodules_path = input_dir.join(".gitmodules");
    let content = std::fs::read_to_string(&gitmodules_path)
        .with_context(|| format!("Failed to read {}", gitmodules_path.display()))?;

    let mut submodules: Vec<SubmoduleInfo> = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_path: Option<String> = None;
    let mut current_url: Option<String> = None;
    let mut current_branch: Option<String> = None;

    for line in content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("[submodule \"") {
            // Flush previous entry
            if let (Some(name), Some(path), Some(url)) =
                (current_name.take(), current_path.take(), current_url.take())
            {
                submodules.push(SubmoduleInfo {
                    name,
                    path,
                    url,
                    branch: current_branch.take().unwrap_or_else(|| "main".into()),
                });
            }
            current_name = rest.strip_suffix("\"]").map(|s| s.to_owned());
        } else if let Some(val) = line.strip_prefix("path = ") {
            current_path = Some(val.to_owned());
        } else if let Some(val) = line.strip_prefix("url = ") {
            current_url = Some(val.to_owned());
        } else if let Some(val) = line.strip_prefix("branch = ") {
            current_branch = Some(val.to_owned());
        }
    }

    // Flush last entry
    if let (Some(name), Some(path), Some(url)) =
        (current_name, current_path, current_url)
    {
        submodules.push(SubmoduleInfo {
            name,
            path,
            url,
            branch: current_branch.unwrap_or_else(|| "main".into()),
        });
    }

    Ok(submodules)
}

// --- Sync state ---

/// Persisted sync state tracking last-synced SHA per submodule.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SyncState {
    #[serde(default)]
    pub submodules: HashMap<String, SubmoduleState>,
}

/// Per-submodule sync state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubmoduleState {
    pub last_sha: String,
    #[serde(default)]
    pub last_sync: Option<String>,
    #[serde(default)]
    pub branch: Option<String>,
}

/// Read `state.json` from the output directory.
pub async fn read_sync_state(output_dir: &Path) -> Result<SyncState> {
    let path = output_dir.join("state.json");
    if !path_exists(&path).await {
        return Ok(SyncState::default());
    }
    let content = read_file(&path).await.context("Failed to read state.json")?;
    serde_json::from_str(&content).context("Failed to parse state.json")
}

/// Atomically write `state.json` (write to .tmp then rename).
pub async fn write_sync_state(output_dir: &Path, state: &SyncState) -> Result<PathBuf> {
    let output_dir = output_dir.to_owned();
    let state = state.clone();
    task::spawn_blocking(move || {
        let path = output_dir.join("state.json");
        let tmp_path = output_dir.join("state.json.tmp");
        let content = serde_json::to_string_pretty(&state)
            .context("Failed to serialize state")?;
        std::fs::write(&tmp_path, &content)
            .with_context(|| format!("Failed to write {}", tmp_path.display()))?;
        std::fs::rename(&tmp_path, &path)
            .with_context(|| format!("Failed to rename tmp to {}", path.display()))?;
        Ok(path)
    })
    .await
    .context("spawn_blocking panicked")?
}

// --- Write operations (with path traversal guards) ---

/// Validate that a resolved path stays within the allowed base directory.
fn validate_path_safety(path: &Path, base_dir: &Path) -> Result<()> {
    // Ensure the parent directory exists so canonicalize works
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }
    let canonical_base = base_dir
        .canonicalize()
        .with_context(|| format!("Failed to canonicalize base dir {}", base_dir.display()))?;
    let canonical_parent = path
        .parent()
        .unwrap_or(path)
        .canonicalize()
        .with_context(|| format!("Failed to canonicalize {}", path.display()))?;
    anyhow::ensure!(
        canonical_parent.starts_with(&canonical_base),
        "Path traversal detected: {} is outside {}",
        path.display(),
        base_dir.display()
    );
    Ok(())
}

/// Write a report markdown file.
///
/// - Base report: `<output>/reports/<date>.md`
/// - Profile report: `<output>/reports/<date>/<profile>.md`
pub async fn write_report(
    output_dir: &Path,
    date: &str,
    content: &str,
    profile: Option<&str>,
) -> Result<PathBuf> {
    let output_dir = output_dir.to_owned();
    let date = date.to_owned();
    let content = content.to_owned();
    let profile = profile.map(|s| s.to_owned());

    task::spawn_blocking(move || {
        let path = match &profile {
            Some(p) => output_dir.join("reports").join(&date).join(format!("{p}.md")),
            None => output_dir.join("reports").join(format!("{date}.md")),
        };
        validate_path_safety(&path, &output_dir)?;
        std::fs::write(&path, &content)
            .with_context(|| format!("Failed to write report {}", path.display()))?;
        Ok(path)
    })
    .await
    .context("spawn_blocking panicked")?
}

/// Write a documentation markdown file.
///
/// - Category doc: `<output>/docs/<name>.md`
/// - Submodule doc: `<output>/docs/submodules/<name>.md`
/// - Profile variant: `<output>/docs/<name>/<profile>.md` or
///   `<output>/docs/submodules/<name>/<profile>.md`
pub async fn write_doc(
    output_dir: &Path,
    name: &str,
    content: &str,
    profile: Option<&str>,
    is_submodule: bool,
) -> Result<PathBuf> {
    let output_dir = output_dir.to_owned();
    let name = name.to_owned();
    let content = content.to_owned();
    let profile = profile.map(|s| s.to_owned());

    task::spawn_blocking(move || {
        let base = if is_submodule {
            output_dir.join("docs").join("submodules")
        } else {
            output_dir.join("docs")
        };
        let path = match &profile {
            Some(p) => base.join(&name).join(format!("{p}.md")),
            None => base.join(format!("{name}.md")),
        };
        validate_path_safety(&path, &output_dir)?;
        std::fs::write(&path, &content)
            .with_context(|| format!("Failed to write doc {}", path.display()))?;
        Ok(path)
    })
    .await
    .context("spawn_blocking panicked")?
}

/// Media manifest entry.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MediaManifestEntry {
    pub filename: String,
    #[serde(rename = "type")]
    pub media_type: String,
    pub repo: String,
    pub pr_number: u64,
    pub pr_title: String,
    pub author: String,
    #[serde(default)]
    pub alt: Option<String>,
    #[serde(default)]
    pub original_url: Option<String>,
}

/// Full media manifest for a date.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MediaManifest {
    pub date: String,
    #[serde(default)]
    pub items: Vec<MediaManifestEntry>,
}

/// Write a media file and update the manifest.json for that date.
pub async fn write_media(
    output_dir: &Path,
    date: &str,
    filename: &str,
    data: &[u8],
    manifest_entry: MediaManifestEntry,
) -> Result<PathBuf> {
    let output_dir = output_dir.to_owned();
    let date = date.to_owned();
    let filename = filename.to_owned();
    let data = data.to_vec();

    task::spawn_blocking(move || {
        let media_dir = output_dir.join("media").join(&date);
        let file_path = media_dir.join(&filename);
        validate_path_safety(&file_path, &output_dir)?;

        // Write the media file
        std::fs::write(&file_path, &data)
            .with_context(|| format!("Failed to write media file {}", file_path.display()))?;

        // Read or create manifest
        let manifest_path = media_dir.join("manifest.json");
        let mut manifest = if manifest_path.is_file() {
            let content = std::fs::read_to_string(&manifest_path)
                .context("Failed to read manifest.json")?;
            serde_json::from_str::<MediaManifest>(&content)
                .unwrap_or(MediaManifest { date: date.clone(), items: Vec::new() })
        } else {
            MediaManifest { date: date.clone(), items: Vec::new() }
        };

        manifest.items.push(manifest_entry);

        let manifest_json = serde_json::to_string_pretty(&manifest)
            .context("Failed to serialize manifest")?;
        std::fs::write(&manifest_path, &manifest_json)
            .with_context(|| format!("Failed to write {}", manifest_path.display()))?;

        Ok(file_path)
    })
    .await
    .context("spawn_blocking panicked")?
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
