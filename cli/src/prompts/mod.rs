//! Prompt templates for MCP prompt handlers.
//!
//! Templates use `{{placeholder}}` markers that are replaced at runtime
//! with configuration values.

use crate::config::ProfileDef;

const SYNC_TEMPLATE: &str = include_str!("sync.md");
const DOCS_TEMPLATE: &str = include_str!("docs.md");

/// Build the sync prompt with configuration values filled in.
pub fn build_sync_prompt(
    input_dir: &str,
    output_dir: &str,
    github_owner: &str,
    timezone: &str,
    date: &str,
    profiles: &[ProfileDef],
) -> String {
    let profiles_text = format_profiles(profiles);

    SYNC_TEMPLATE
        .replace("{{input_dir}}", input_dir)
        .replace("{{output_dir}}", output_dir)
        .replace("{{github_owner}}", github_owner)
        .replace("{{timezone}}", timezone)
        .replace("{{date}}", date)
        .replace("{{profiles}}", &profiles_text)
}

/// Build the docs prompt with configuration values filled in.
pub fn build_docs_prompt(
    input_dir: &str,
    output_dir: &str,
    github_owner: &str,
    doc_categories: &[String],
    submodule: Option<&str>,
    full: bool,
    profiles: &[ProfileDef],
) -> String {
    let profiles_text = format_profiles(profiles);
    let categories_text = doc_categories.join(", ");

    let scope = match (submodule, full) {
        (Some(name), _) => format!("Single submodule: `{name}`"),
        (None, true) => "Full regeneration (all submodules, ignore timestamps)".to_owned(),
        (None, false) => "Incremental (only submodules with changes since last update)".to_owned(),
    };

    DOCS_TEMPLATE
        .replace("{{input_dir}}", input_dir)
        .replace("{{output_dir}}", output_dir)
        .replace("{{github_owner}}", github_owner)
        .replace("{{doc_categories}}", &categories_text)
        .replace("{{scope}}", &scope)
        .replace("{{profiles}}", &profiles_text)
}

fn format_profiles(profiles: &[ProfileDef]) -> String {
    profiles
        .iter()
        .map(|p| {
            format!(
                "- **{}** (`{}`): {}\n  Focus: {}",
                p.label,
                p.id,
                p.description,
                p.focus.join(", ")
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}
