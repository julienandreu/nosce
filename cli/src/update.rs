use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

const LATEST_URL: &str =
    "https://github.com/julienandreu/nosce/releases/latest/download/latest.json";
const RELEASE_DOWNLOAD_BASE: &str =
    "https://github.com/julienandreu/nosce/releases/latest/download";
const CHECK_INTERVAL: Duration = Duration::from_secs(86400);

fn http_client(timeout: Duration) -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent(concat!("nosce/", env!("CARGO_PKG_VERSION")))
        .timeout(timeout)
        .build()
        .context("Failed to create HTTP client")
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReleaseManifest {
    pub version: String,
    pub commit: String,
    pub date: String,
    pub files: HashMap<String, ReleaseFile>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReleaseFile {
    pub name: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct UpdateCache {
    last_check: String,
    manifest: ReleaseManifest,
}

pub enum UpdateStatus {
    UpToDate,
    NewVersion {
        current: String,
        latest: String,
    },
    NewBuild {
        version: String,
        current_commit: String,
        latest_commit: String,
    },
}

fn cache_path() -> PathBuf {
    super::nosce_home().join("update-check.json")
}

fn is_stderr_tty() -> bool {
    use std::io::IsTerminal;
    std::io::stderr().is_terminal()
}

const fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

const fn current_commit() -> &'static str {
    env!("NOSCE_COMMIT_HASH")
}

const fn current_target() -> &'static str {
    env!("NOSCE_TARGET")
}

fn short_commit(hash: &str) -> &str {
    &hash[..hash.len().min(7)]
}

fn version_is_greater(current: &str, latest: &str) -> bool {
    let parse = |s: &str| -> Option<(u64, u64, u64)> {
        let mut parts = s.split('.');
        let major = parts.next()?.parse().ok()?;
        let minor = parts.next()?.parse().ok()?;
        let patch = parts.next()?.parse().ok()?;
        Some((major, minor, patch))
    };

    matches!((parse(current), parse(latest)), (Some(c), Some(l)) if l > c)
}

fn check_status(manifest: &ReleaseManifest) -> UpdateStatus {
    let cv = current_version();
    let cc = current_commit();

    if version_is_greater(cv, &manifest.version) {
        return UpdateStatus::NewVersion {
            current: cv.to_string(),
            latest: manifest.version.clone(),
        };
    }

    let same_version = cv == manifest.version;
    let commits_differ = !cc.is_empty() && !manifest.commit.is_empty() && cc != manifest.commit;

    if same_version && commits_differ {
        return UpdateStatus::NewBuild {
            version: manifest.version.clone(),
            current_commit: short_commit(cc).to_owned(),
            latest_commit: short_commit(&manifest.commit).to_owned(),
        };
    }

    UpdateStatus::UpToDate
}

fn read_cache() -> Option<UpdateCache> {
    let data = std::fs::read_to_string(cache_path()).ok()?;
    serde_json::from_str(&data).ok()
}

fn write_cache(manifest: &ReleaseManifest) {
    let cache = UpdateCache {
        last_check: chrono::Utc::now().to_rfc3339(),
        manifest: manifest.clone(),
    };
    let Ok(json) = serde_json::to_string_pretty(&cache) else {
        return;
    };
    let path = cache_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, json);
}

fn cache_is_fresh(cache: &UpdateCache) -> bool {
    chrono::DateTime::parse_from_rfc3339(&cache.last_check)
        .ok()
        .map(|dt| dt.with_timezone(&chrono::Utc))
        .is_some_and(|ts| {
            let max_age = chrono::Duration::from_std(CHECK_INTERVAL)
                .unwrap_or_else(|_| chrono::Duration::hours(24));
            chrono::Utc::now().signed_duration_since(ts) < max_age
        })
}

async fn fetch_manifest() -> Result<ReleaseManifest> {
    http_client(Duration::from_secs(10))?
        .get(LATEST_URL)
        .send()
        .await
        .context("Failed to fetch latest.json")?
        .error_for_status()
        .context("GitHub returned an error")?
        .json()
        .await
        .context("Failed to parse latest.json")
}

pub async fn check_for_update_bg() {
    if !is_stderr_tty() {
        return;
    }

    let result = async {
        if let Some(cache) = read_cache().filter(cache_is_fresh) {
            return Ok(check_status(&cache.manifest));
        }

        let manifest = fetch_manifest().await?;
        let status = check_status(&manifest);
        write_cache(&manifest);
        Ok::<_, anyhow::Error>(status)
    }
    .await;

    match result {
        Ok(UpdateStatus::NewVersion { current, latest }) => {
            eprintln!(
                "\x1b[33mUpdate available: v{current} -> v{latest} \
                 -- run `nosce update` to install\x1b[0m"
            );
        }
        Ok(UpdateStatus::NewBuild {
            version,
            current_commit,
            latest_commit,
        }) => {
            eprintln!(
                "\x1b[33mNewer build of v{version} available \
                 ({current_commit} -> {latest_commit}) \
                 -- run `nosce update`\x1b[0m"
            );
        }
        Ok(UpdateStatus::UpToDate) => {}
        Err(e) => {
            tracing::debug!("Background update check failed: {e:#}");
        }
    }
}

pub fn run_update(check_only: bool) -> Result<()> {
    let rt = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;

    rt.block_on(async {
        let manifest = fetch_manifest().await?;
        let status = check_status(&manifest);
        write_cache(&manifest);

        if check_only {
            print_check_result(&manifest, &status);
            return Ok(());
        }

        match status {
            UpdateStatus::UpToDate => {
                eprintln!(
                    "nosce v{} ({}) is up to date.",
                    current_version(),
                    short_commit(current_commit()),
                );
                Ok(())
            }
            UpdateStatus::NewVersion { current, latest } => {
                eprintln!("Updating nosce v{current} -> v{latest}...");
                do_update(&manifest).await
            }
            UpdateStatus::NewBuild {
                version,
                current_commit,
                latest_commit,
            } => {
                eprintln!("Updating nosce v{version} ({current_commit} -> {latest_commit})...");
                do_update(&manifest).await
            }
        }
    })
}

fn print_check_result(manifest: &ReleaseManifest, status: &UpdateStatus) {
    eprintln!("nosce v{}", current_version());
    eprintln!(
        "  commit:  {} ({})",
        short_commit(current_commit()),
        env!("NOSCE_COMMIT_DATE"),
    );
    eprintln!("  built:   {}", env!("NOSCE_BUILD_TIMESTAMP"));
    eprintln!("  target:  {}", current_target());
    eprintln!();

    eprintln!(
        "Latest: v{} ({}, {})",
        manifest.version,
        short_commit(&manifest.commit),
        manifest.date,
    );

    match status {
        UpdateStatus::UpToDate => eprintln!("Status: Up to date"),
        UpdateStatus::NewVersion { .. } => eprintln!("Status: Update available"),
        UpdateStatus::NewBuild { .. } => eprintln!("Status: Newer build available"),
    }
}

async fn do_update(manifest: &ReleaseManifest) -> Result<()> {
    let target = current_target();
    let file_info = manifest
        .files
        .get(target)
        .with_context(|| format!("No release asset found for target: {target}"))?;

    let download_url = format!("{RELEASE_DOWNLOAD_BASE}/{}", file_info.name);

    eprintln!("Downloading {}...", file_info.name);

    let bytes = http_client(Duration::from_secs(300))?
        .get(&download_url)
        .send()
        .await
        .context("Failed to download release")?
        .error_for_status()
        .context("GitHub returned an error for download")?
        .bytes()
        .await
        .context("Failed to read download body")?;

    let digest = {
        use sha2::Digest;
        let mut hasher = sha2::Sha256::new();
        hasher.update(&bytes);
        format!("{:x}", hasher.finalize())
    };

    if digest != file_info.sha256 {
        anyhow::bail!(
            "Checksum mismatch: expected {}, got {digest}",
            file_info.sha256,
        );
    }
    eprintln!("Checksum verified.");

    let decoder = flate2::read::GzDecoder::new(&bytes[..]);
    let mut archive = tar::Archive::new(decoder);

    let tmp_dir = tempfile::tempdir().context("Failed to create temp directory")?;
    let mut extracted_binary = None;

    for entry_result in archive.entries().context("Failed to read tar entries")? {
        let mut entry = entry_result.context("Failed to read tar entry")?;
        let path = entry.path().context("Failed to get entry path")?;

        if path.file_name().and_then(|n| n.to_str()) == Some("nosce") {
            let dest = tmp_dir.path().join("nosce");
            entry.unpack(&dest).context("Failed to extract binary")?;
            extracted_binary = Some(dest);
            break;
        }
    }

    let new_binary = extracted_binary.context("Binary 'nosce' not found in archive")?;

    self_replace::self_replace(&new_binary).context("Failed to replace current binary")?;

    eprintln!(
        "Updated to nosce v{} ({}).",
        manifest.version,
        short_commit(&manifest.commit),
    );
    Ok(())
}
