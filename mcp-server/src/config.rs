use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Full settings parsed from nosce.yml.
#[derive(Debug, Clone)]
pub struct NosceSettings {
    pub input_dir: Option<PathBuf>,
    pub output_dir: Option<PathBuf>,
    pub github_owner: Option<String>,
    pub timezone: Option<String>,
    pub doc_categories: Vec<String>,
    pub profiles: Vec<ProfileDef>,
}

/// Raw YAML structure for deserialization.
#[derive(Debug, Deserialize)]
struct NosceConfig {
    #[serde(default)]
    input: Option<String>,
    #[serde(default)]
    output: Option<String>,
    #[serde(default)]
    github_owner: Option<String>,
    #[serde(default)]
    reports: Option<ReportsConfig>,
    #[serde(default)]
    docs: Option<DocsConfig>,
    #[serde(default)]
    profiles: Vec<ProfileDef>,
}

#[derive(Debug, Deserialize)]
struct ReportsConfig {
    #[serde(default)]
    timezone: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DocsConfig {
    #[serde(default)]
    categories: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProfileDef {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub description: String,
    pub focus: Vec<String>,
}

/// Load the full settings from nosce.yml. Returns sensible defaults if the
/// file is missing or fields are absent.
pub fn load_settings(config_path: &Path) -> NosceSettings {
    let content = match std::fs::read_to_string(config_path) {
        Ok(c) => c,
        Err(_) => {
            tracing::info!(
                "Config not found at {}, using defaults",
                config_path.display()
            );
            return NosceSettings {
                input_dir: None,
                output_dir: None,
                github_owner: None,
                timezone: None,
                doc_categories: default_doc_categories(),
                profiles: default_profiles(),
            };
        }
    };

    match serde_yaml::from_str::<NosceConfig>(&content) {
        Ok(cfg) => NosceSettings {
            input_dir: cfg.input.map(|s| PathBuf::from(shellexpand::tilde(&s).into_owned())),
            output_dir: cfg.output.map(|s| PathBuf::from(shellexpand::tilde(&s).into_owned())),
            github_owner: cfg.github_owner,
            timezone: cfg.reports.and_then(|r| r.timezone),
            doc_categories: cfg
                .docs
                .map(|d| {
                    if d.categories.is_empty() {
                        default_doc_categories()
                    } else {
                        d.categories
                    }
                })
                .unwrap_or_else(default_doc_categories),
            profiles: if cfg.profiles.is_empty() {
                default_profiles()
            } else {
                cfg.profiles
            },
        },
        Err(err) => {
            tracing::warn!("Failed to parse config: {err}, using defaults");
            NosceSettings {
                input_dir: None,
                output_dir: None,
                github_owner: None,
                timezone: None,
                doc_categories: default_doc_categories(),
                profiles: default_profiles(),
            }
        }
    }
}


fn default_doc_categories() -> Vec<String> {
    vec![
        "overview".into(),
        "architecture".into(),
        "apis".into(),
        "databases".into(),
        "dependencies".into(),
    ]
}

fn default_profiles() -> Vec<ProfileDef> {
    vec![
        ProfileDef {
            id: "engineer".into(),
            label: "Engineer".into(),
            icon: "wrench".into(),
            description: "Technical depth: code changes, diffs, architectural impact".into(),
            focus: vec![
                "commit_details".into(),
                "code_diffs".into(),
                "breaking_changes".into(),
                "tech_debt".into(),
                "architectural_impact".into(),
            ],
        },
        ProfileDef {
            id: "pm".into(),
            label: "Product Manager".into(),
            icon: "clipboard".into(),
            description: "Feature progress, user-facing changes, delivery timeline".into(),
            focus: vec![
                "feature_progress".into(),
                "user_facing_changes".into(),
                "delivery_status".into(),
                "blockers".into(),
                "sprint_alignment".into(),
            ],
        },
        ProfileDef {
            id: "cto".into(),
            label: "CTO / CEO".into(),
            icon: "chart".into(),
            description: "Executive summary: strategic impact, platform health, key metrics"
                .into(),
            focus: vec![
                "executive_summary".into(),
                "strategic_impact".into(),
                "platform_health".into(),
                "key_metrics".into(),
                "business_risk".into(),
            ],
        },
    ]
}
