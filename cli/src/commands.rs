use std::io::Read as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};

pub fn run_init() -> Result<()> {
    use dialoguer::{theme::ColorfulTheme, Confirm, Input};

    use crate::config;

    let theme = ColorfulTheme::default();
    let cwd = std::env::current_dir().context("Failed to get current directory")?;
    let config_path = cwd.join("nosce.config.yml");

    eprintln!();
    eprintln!("  {}", style("nosce init").bold().cyan());
    eprintln!("  {}", style("───────────────────────────────").dim());
    eprintln!();

    // Load existing config for pre-filling defaults
    let existing = config::load_settings(&config_path);

    if config_path.exists() {
        eprintln!(
            "  {} Found existing nosce.config.yml — values will be used as defaults.",
            style("i").cyan().bold()
        );
        eprintln!();

        let overwrite = Confirm::with_theme(&theme)
            .with_prompt("Overwrite nosce.config.yml?")
            .default(true)
            .interact()
            .context("Failed to read input")?;

        if !overwrite {
            eprintln!("  Aborted.");
            eprintln!();
            return Ok(());
        }
    }

    // -- Directories (pre-fill from existing config) --

    let default_input = existing
        .input_dir
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| {
            home_dir()
                .join(".nosce")
                .join("input")
                .display()
                .to_string()
        });

    let input_dir: String = Input::with_theme(&theme)
        .with_prompt("Input directory (git repo with submodules)")
        .default(default_input)
        .interact_text()
        .context("Failed to read input")?;

    let default_output = existing
        .output_dir
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| {
            home_dir()
                .join(".nosce")
                .join("output")
                .display()
                .to_string()
        });

    let output_dir: String = Input::with_theme(&theme)
        .with_prompt("Output directory")
        .default(default_output)
        .interact_text()
        .context("Failed to read input")?;

    let input_dir = resolve_to_absolute(&cwd, &input_dir);
    let output_dir = resolve_to_absolute(&cwd, &output_dir);

    // -- GitHub owner --

    let github_owner: String = Input::with_theme(&theme)
        .with_prompt("GitHub owner (org or user)")
        .default(existing.github_owner.unwrap_or_default())
        .allow_empty(true)
        .interact_text()
        .context("Failed to read input")?;

    // -- Timezone --

    let tz_default = existing
        .timezone
        .unwrap_or_else(|| iana_timezone().unwrap_or_else(|| "UTC".into()));

    let timezone: String = Input::with_theme(&theme)
        .with_prompt("Timezone")
        .default(tz_default)
        .interact_text()
        .context("Failed to read input")?;

    // -- Create output directory tree --

    let output_path = PathBuf::from(&output_dir);

    std::fs::create_dir_all(&output_path)
        .with_context(|| format!("Failed to create directory: {}", output_path.display()))?;

    for sub in &["docs", "docs/submodules", "reports", "media"] {
        std::fs::create_dir_all(output_path.join(sub))?;
    }

    eprintln!();
    eprintln!(
        "  {} Created {}",
        style("✓").green().bold(),
        style(output_path.display()).underlined()
    );

    // -- Preserve profiles and categories from existing config --

    let categories = existing.doc_categories;
    let profiles = existing.profiles;

    // -- Write nosce.config.yml --

    let github_line = if github_owner.is_empty() {
        "# github_owner: your-org".to_owned()
    } else {
        format!("github_owner: {github_owner}")
    };

    let categories_yaml = categories
        .iter()
        .map(|c| format!("    - {c}"))
        .collect::<Vec<_>>()
        .join("\n");

    let profiles_yaml = profiles
        .iter()
        .map(format_profile_yaml)
        .collect::<Vec<_>>()
        .join("\n");

    let yaml = format!(
        "version: 1\n\
         \n\
         # Paths (overridable via CLI flags or env vars)\n\
         input: {input_dir}\n\
         output: {output_dir}\n\
         \n\
         # GitHub owner for gh CLI (PR lookups)\n\
         {github_line}\n\
         \n\
         # Report settings\n\
         reports:\n\
         \x20 timezone: {timezone}\n\
         \n\
         # Docs settings\n\
         docs:\n\
         \x20 categories:\n\
         {categories_yaml}\n\
         \n\
         # User profiles — each profile sees a tailored report summary\n\
         profiles:\n\
         {profiles_yaml}\n"
    );

    std::fs::write(&config_path, &yaml)
        .with_context(|| format!("Failed to write {}", config_path.display()))?;

    eprintln!(
        "  {} Wrote {} ({} profiles, {} doc categories)",
        style("✓").green().bold(),
        style(config_path.display()).underlined(),
        profiles.len(),
        categories.len(),
    );

    // -- Shell env var instructions --

    let shell = std::env::var("SHELL").unwrap_or_default();
    let shell_name = Path::new(&shell)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("sh");

    let rc_file = match shell_name {
        "zsh" => "~/.zshrc",
        "bash" => "~/.bashrc",
        "fish" => "~/.config/fish/config.fish",
        "nu" | "nushell" => "~/.config/nushell/env.nu",
        _ => "~/.profile",
    };

    let export_line = match shell_name {
        "fish" => format!("set -gx NOSCE_OUTPUT_DIR \"{output_dir}\""),
        "nu" | "nushell" => format!("$env.NOSCE_OUTPUT_DIR = \"{output_dir}\""),
        _ => format!("export NOSCE_OUTPUT_DIR=\"{output_dir}\""),
    };

    eprintln!();
    eprintln!(
        "  Optionally, add this to your {} so you can skip {}:",
        style(rc_file).bold().yellow(),
        style("--output-dir").dim(),
    );
    eprintln!();
    eprintln!("    {}", style(&export_line).green());
    eprintln!();
    eprintln!("  Then reload:");
    eprintln!();
    eprintln!("    {}", style(format!("source {rc_file}")).dim());
    eprintln!();

    eprintln!(
        "  {} Review and edit {} to customize profiles,",
        style("→").cyan().bold(),
        style("nosce.config.yml").bold(),
    );
    eprintln!("    doc categories, and other settings.");
    eprintln!();

    Ok(())
}

fn format_profile_yaml(profile: &crate::config::ProfileDef) -> String {
    let focus_lines = profile
        .focus
        .iter()
        .map(|f| format!("      - {f}"))
        .collect::<Vec<_>>()
        .join("\n");

    // Escape description for YAML (wrap in quotes if it contains colons)
    let desc = if profile.description.contains(':') {
        format!("|\n      {}", profile.description.trim())
    } else {
        profile.description.clone()
    };

    format!(
        "  - id: {}\n    label: {}\n    icon: {}\n    description: {}\n    focus:\n{}",
        profile.id, profile.label, profile.icon, desc, focus_lines
    )
}

fn iana_timezone() -> Option<String> {
    // macOS: read /etc/localtime symlink target
    if let Ok(target) = std::fs::read_link("/etc/localtime") {
        let s = target.to_string_lossy();
        if let Some(tz) = s.strip_prefix("/var/db/timezone/zoneinfo/") {
            return Some(tz.to_owned());
        }
        if let Some(tz) = s.strip_prefix("/usr/share/zoneinfo/") {
            return Some(tz.to_owned());
        }
    }
    // Linux: /etc/timezone
    if let Ok(tz) = std::fs::read_to_string("/etc/timezone") {
        let trimmed = tz.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_owned());
        }
    }
    // TZ env var fallback
    std::env::var("TZ").ok().filter(|s| !s.is_empty())
}

pub fn run_setup_mcp() -> Result<()> {
    use dialoguer::{theme::ColorfulTheme, Confirm};

    let theme = ColorfulTheme::default();
    let cwd = std::env::current_dir().context("Failed to get current directory")?;
    let mcp_path = cwd.join(".mcp.json");

    eprintln!();
    eprintln!("  {}", style("nosce setup-mcp").bold().cyan());
    eprintln!("  {}", style("───────────────────────────────").dim());
    eprintln!();

    if mcp_path.exists() {
        let overwrite = Confirm::with_theme(&theme)
            .with_prompt(".mcp.json already exists — overwrite?")
            .default(false)
            .interact()
            .context("Failed to read input")?;

        if !overwrite {
            eprintln!("  Aborted.");
            eprintln!();
            return Ok(());
        }
    }

    let nosce_bin = std::env::current_exe()
        .context("Failed to determine nosce binary path")?
        .canonicalize()
        .context("Failed to canonicalize binary path")?
        .display()
        .to_string();

    // Try to read output_dir from nosce.config.yml in PWD, then env var, then default
    let output_dir = resolve_mcp_output_dir(&cwd);

    let mcp_json = serde_json::json!({
        "mcpServers": {
            "nosce": {
                "type": "stdio",
                "command": nosce_bin,
                "args": ["mcp", "--output-dir", output_dir]
            }
        }
    });

    let content =
        serde_json::to_string_pretty(&mcp_json).context("Failed to serialize .mcp.json")?;

    std::fs::write(&mcp_path, format!("{content}\n"))
        .with_context(|| format!("Failed to write {}", mcp_path.display()))?;

    eprintln!(
        "  {} Wrote {}",
        style("✓").green().bold(),
        style(mcp_path.display()).underlined()
    );
    eprintln!();
    eprintln!("  {}", style("Configuration:").dim());
    eprintln!("    binary:     {}", style(&nosce_bin).green());
    eprintln!("    output-dir: {}", style(&output_dir).green());
    eprintln!();

    let config_path = cwd.join("nosce.config.yml");
    if !config_path.exists() {
        eprintln!(
            "  {} No nosce.config.yml found — run {} first for full setup.",
            style("!").yellow().bold(),
            style("nosce init").bold(),
        );
        eprintln!();
    }

    Ok(())
}

fn resolve_mcp_output_dir(cwd: &Path) -> String {
    // 1. Try nosce.config.yml in PWD
    let config_path = cwd.join("nosce.config.yml");
    if let Ok(content) = std::fs::read_to_string(&config_path) {
        if let Ok(yaml) = serde_yaml::from_str::<serde_yaml::Value>(&content) {
            if let Some(output) = yaml.get("output").and_then(|v| v.as_str()) {
                let expanded = shellexpand::tilde(output).to_string();
                let path = PathBuf::from(&expanded);
                if path.is_absolute() {
                    return expanded;
                }
                return cwd.join(&path).display().to_string();
            }
        }
    }

    // 2. Try NOSCE_OUTPUT_DIR env var
    if let Ok(val) = std::env::var("NOSCE_OUTPUT_DIR") {
        if !val.is_empty() {
            let expanded = shellexpand::tilde(&val).to_string();
            return expanded;
        }
    }

    // 3. Default
    home_dir()
        .join(".nosce")
        .join("output")
        .display()
        .to_string()
}

pub fn run_export(output_dir: &Path, target: &str) -> Result<()> {
    let target_path = PathBuf::from(shellexpand::tilde(target).as_ref());

    if target_path.extension().and_then(|e| e.to_str()) != Some("zip") {
        anyhow::bail!(
            "Target must have a .zip extension, got: {}",
            target_path.display()
        );
    }

    eprintln!();
    eprintln!(
        "  {} {}",
        style("Exporting").bold().cyan(),
        output_dir.display()
    );
    eprintln!("  {} {}", style("→").dim(), target_path.display());
    eprintln!();

    let mut files: Vec<PathBuf> = Vec::new();
    for entry in walkdir::WalkDir::new(output_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            files.push(entry.into_path());
        }
    }

    if files.is_empty() {
        eprintln!(
            "  {} Output directory is empty, nothing to export.",
            style("!").yellow().bold()
        );
        eprintln!();
        return Ok(());
    }

    let pb = ProgressBar::new(files.len() as u64);
    pb.set_style(
        ProgressStyle::with_template("  {spinner:.green} [{bar:30.cyan/dim}] {pos}/{len} {msg}")?
            .progress_chars("━╸─"),
    );

    if let Some(parent) = target_path.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create parent directory: {}", parent.display()))?;
    }

    let file = std::fs::File::create(&target_path)
        .with_context(|| format!("Failed to create {}", target_path.display()))?;

    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for file_path in &files {
        let relative = file_path.strip_prefix(output_dir).unwrap_or(file_path);
        let name = relative.to_string_lossy();

        pb.set_message(truncate_path(&name, 40));

        zip.start_file(name.as_ref(), options)
            .with_context(|| format!("Failed to add {name} to archive"))?;

        let mut f = std::fs::File::open(file_path)
            .with_context(|| format!("Failed to read {}", file_path.display()))?;
        std::io::copy(&mut f, &mut zip).with_context(|| format!("Failed to write {name}"))?;

        pb.inc(1);
    }

    zip.finish().context("Failed to finalize zip archive")?;
    pb.finish_and_clear();

    let size = std::fs::metadata(&target_path)
        .map(|m| m.len())
        .unwrap_or(0);

    eprintln!(
        "  {} Exported {} files ({}) to {}",
        style("✓").green().bold(),
        files.len(),
        style(format_size(size)).dim(),
        style(target_path.display()).underlined(),
    );
    eprintln!();

    Ok(())
}

pub fn run_import(output_dir: &Path, source: &str) -> Result<()> {
    let source_path = PathBuf::from(shellexpand::tilde(source).as_ref());

    if !source_path.is_file() {
        anyhow::bail!("Source file not found: {}", source_path.display());
    }

    eprintln!();
    eprintln!(
        "  {} {}",
        style("Importing").bold().cyan(),
        source_path.display()
    );
    eprintln!("  {} {}", style("→").dim(), output_dir.display());
    eprintln!();

    let file = std::fs::File::open(&source_path)
        .with_context(|| format!("Failed to open {}", source_path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("Invalid zip archive: {}", source_path.display()))?;

    let total = archive.len();

    let pb = ProgressBar::new(total as u64);
    pb.set_style(
        ProgressStyle::with_template("  {spinner:.green} [{bar:30.cyan/dim}] {pos}/{len} {msg}")?
            .progress_chars("━╸─"),
    );

    std::fs::create_dir_all(output_dir)
        .with_context(|| format!("Failed to create {}", output_dir.display()))?;

    let canonical_base = output_dir
        .canonicalize()
        .context("Failed to canonicalize output dir")?;

    let mut imported = 0u64;

    for i in 0..total {
        let mut entry = archive
            .by_index(i)
            .with_context(|| format!("Failed to read zip entry {i}"))?;

        let name = entry.name().to_owned();
        pb.set_message(truncate_path(&name, 40));

        let out_path = output_dir.join(&name);

        if entry.is_dir() {
            std::fs::create_dir_all(&out_path)?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let canonical_parent = out_path
                .parent()
                .unwrap_or(&out_path)
                .canonicalize()
                .unwrap_or_else(|_| out_path.clone());

            if !canonical_parent.starts_with(&canonical_base) {
                tracing::warn!("Skipping path traversal attempt: {name}");
                pb.inc(1);
                continue;
            }

            let mut buf = Vec::new();
            entry
                .read_to_end(&mut buf)
                .with_context(|| format!("Failed to read {name}"))?;
            std::fs::write(&out_path, &buf)
                .with_context(|| format!("Failed to write {}", out_path.display()))?;
            imported += 1;
        }

        pb.inc(1);
    }

    pb.finish_and_clear();

    eprintln!(
        "  {} Imported {} files to {}",
        style("✓").green().bold(),
        imported,
        style(output_dir.display()).underlined(),
    );
    eprintln!();

    Ok(())
}

/// Resolve a user-provided path to an absolute path.
/// Expands `~`, then resolves relative paths against `cwd`.
fn resolve_to_absolute(cwd: &Path, raw: &str) -> String {
    let expanded = shellexpand::tilde(raw).to_string();
    let path = PathBuf::from(&expanded);
    if path.is_absolute() {
        expanded
    } else {
        cwd.join(&path).display().to_string()
    }
}

fn home_dir() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        return path.to_string();
    }
    let suffix = &path[path.len() - (max_len - 3)..];
    format!("...{suffix}")
}
