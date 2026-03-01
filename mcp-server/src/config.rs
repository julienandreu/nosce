use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct NosceConfig {
    #[serde(default)]
    profiles: Vec<ProfileDef>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProfileDef {
    pub id: String,
    pub label: String,
    pub icon: String,
    pub description: String,
    pub focus: Vec<String>,
}

/// Load profile definitions from nosce.yml. Returns defaults if the file
/// is missing or the `profiles` key is absent.
pub fn load_profiles(config_path: &Path) -> Vec<ProfileDef> {
    let content = match std::fs::read_to_string(config_path) {
        Ok(c) => c,
        Err(_) => {
            tracing::info!(
                "Config not found at {}, using default profiles",
                config_path.display()
            );
            return default_profiles();
        }
    };

    match serde_yaml::from_str::<NosceConfig>(&content) {
        Ok(cfg) if !cfg.profiles.is_empty() => cfg.profiles,
        Ok(_) => {
            tracing::info!("No profiles in config, using defaults");
            default_profiles()
        }
        Err(err) => {
            tracing::warn!("Failed to parse config: {err}, using default profiles");
            default_profiles()
        }
    }
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
