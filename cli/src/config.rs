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
            input_dir: cfg
                .input
                .map(|s| PathBuf::from(shellexpand::tilde(&s).into_owned())),
            output_dir: cfg
                .output
                .map(|s| PathBuf::from(shellexpand::tilde(&s).into_owned())),
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

pub fn default_doc_categories() -> Vec<String> {
    vec![
        "overview".into(),
        "customer experience".into(),
        "architecture".into(),
        "apis".into(),
        "databases".into(),
        "dependencies".into(),
    ]
}

pub fn default_profiles() -> Vec<ProfileDef> {
    vec![
        ProfileDef {
            id: "engineer".into(),
            label: "Engineer".into(),
            icon: "wrench".into(),
            description: "Technical implementation: code changes, diffs, breaking changes, architecture, testing, deployment risks".into(),
            focus: vec![
                "commit_details".into(),
                "code_diffs".into(),
                "breaking_changes".into(),
                "tech_debt".into(),
                "architectural_impact".into(),
                "test_coverage_impact".into(),
                "regression_potential".into(),
                "deployment_safety".into(),
            ],
        },
        ProfileDef {
            id: "fde".into(),
            label: "Forward Deployed Engineer".into(),
            icon: "robot".into(),
            description: "Forward-deployed perspective: customer-facing changes, deployment impact, API stability, integration risks, and demo readiness".into(),
            focus: vec![
                "customer_impact".into(),
                "api_changes".into(),
                "integration_risks".into(),
                "commit_details".into(),
                "code_diffs".into(),
                "breaking_changes".into(),
                "regression_potential".into(),
                "deployment_safety".into(),
                "screenshots".into(),
                "videos".into(),
                "code_review".into(),
                "code_review_comments".into(),
            ],
        },
        ProfileDef {
            id: "product".into(),
            label: "Product".into(),
            icon: "lightbulb".into(),
            description: "Delivery orchestration: feature progress, dependencies, velocity, risk assessment, roadmap alignment".into(),
            focus: vec![
                "feature_progress".into(),
                "delivery_status".into(),
                "blockers".into(),
                "cross_team_dependencies".into(),
                "risk_assessment".into(),
                "sprint_health".into(),
                "roadmap_alignment".into(),
                "feature_completeness".into(),
                "screenshots".into(),
                "videos".into(),
            ],
        },
        ProfileDef {
            id: "marketing".into(),
            label: "Marketing".into(),
            icon: "rocket".into(),
            description: "Marketing perspective: user-facing improvements, feature announcements, visual changes, and competitive positioning".into(),
            focus: vec![
                "release_highlights".into(),
                "new_features".into(),
                "customer_benefits".into(),
                "competitive_advantages".into(),
                "user_facing_changes".into(),
                "user_facing_bugs".into(),
                "ux_improvements".into(),
                "demo_worthy_changes".into(),
                "screenshots".into(),
                "videos".into(),
            ],
        },
        ProfileDef {
            id: "sales".into(),
            label: "Sales".into(),
            icon: "megaphone".into(),
            description: "Customer-facing changes: new features, bug fixes, competitive advantages, answers for sales leads".into(),
            focus: vec![
                "new_features".into(),
                "customer_benefits".into(),
                "competitive_advantages".into(),
                "release_highlights".into(),
                "user_facing_bugs".into(),
                "ux_improvements".into(),
                "demo_worthy_changes".into(),
                "screenshots".into(),
                "videos".into(),
            ],
        },
    ]
}
