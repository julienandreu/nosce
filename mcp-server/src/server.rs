use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rmcp::{
    handler::server::router::prompt::PromptRouter,
    handler::server::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    model::{
        AnnotateAble, CallToolResult, Content, GetPromptRequestParams, GetPromptResult,
        Implementation, ListPromptsResult, ListResourcesResult, PaginatedRequestParams,
        PromptMessage, PromptMessageRole, ProtocolVersion, RawResource,
        ReadResourceRequestParams, ReadResourceResult, ResourceContents, ServerCapabilities,
        ServerInfo,
    },
    prompt, prompt_handler, prompt_router, service::RequestContext, tool, tool_handler,
    tool_router, ErrorData as McpError, RoleServer, ServerHandler,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::config::ProfileDef;
use crate::fs_ops;
use crate::prompts;

#[derive(Clone)]
pub struct NosceServer {
    output_dir: PathBuf,
    input_dir: Option<PathBuf>,
    github_owner: Option<String>,
    doc_categories: Vec<String>,
    timezone: Option<String>,
    profiles: Vec<ProfileDef>,
    tool_router: ToolRouter<Self>,
    prompt_router: PromptRouter<Self>,
}

// -- Tool parameter types (read-only tools) --

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetDailyReportParams {
    /// Date of the report in YYYY-MM-DD format. If omitted, returns the latest report.
    pub date: Option<String>,
    /// Profile ID for a role-specific view (e.g., "engineer", "cto", "pm").
    /// If omitted, returns the full base report.
    pub profile: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListProfilesParams {}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListReportsParams {
    /// Maximum number of reports to return (default: 30)
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetDocParams {
    /// Document category: overview, architecture, apis, databases, or dependencies
    pub category: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchDocsParams {
    /// Search query (case-insensitive substring match)
    pub query: String,
    /// Maximum number of results to return (default: 10)
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetSubmoduleDocParams {
    /// Name of the submodule
    pub name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetChangelogParams {
    /// Name of the submodule
    pub name: String,
    /// Start date in YYYY-MM-DD format (inclusive)
    pub from: Option<String>,
    /// End date in YYYY-MM-DD format (inclusive)
    pub to: Option<String>,
}

// -- Tool parameter types (write tools) --

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DiscoverSubmodulesParams {
    /// Override the input directory (uses server default if omitted)
    pub input_dir: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetSyncStateParams {}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WriteReportParams {
    /// Report date in YYYY-MM-DD format
    pub date: String,
    /// Markdown content of the report
    pub content: String,
    /// Profile ID for a profile-specific report. If omitted, writes the base report.
    pub profile: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WriteDocParams {
    /// Document name (category name like "overview", or submodule name)
    pub name: String,
    /// Markdown content of the documentation
    pub content: String,
    /// Whether this is a submodule doc (true) or a category doc (false)
    pub is_submodule: bool,
    /// Profile ID for a profile-specific variant. If omitted, writes the base doc.
    pub profile: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
pub struct SubmoduleStateUpdate {
    /// Latest commit SHA
    pub last_sha: String,
    /// Branch name
    pub branch: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateSyncStateParams {
    /// Map of submodule name to its updated state (last_sha, branch)
    pub submodules: HashMap<String, SubmoduleStateUpdate>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WriteMediaParams {
    /// Report date in YYYY-MM-DD format
    pub date: String,
    /// Filename for the media file (e.g., "repo-pr42-1.png")
    pub filename: String,
    /// Base64-encoded file content
    pub data: String,
    /// Metadata for the manifest entry
    pub manifest_entry: fs_ops::MediaManifestEntry,
}

// -- Prompt parameter types --

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SyncPromptParams {
    /// Target date in YYYY-MM-DD format. Defaults to today.
    pub date: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocsPromptParams {
    /// If provided, only generate docs for this specific submodule
    pub submodule: Option<String>,
    /// Force full regeneration, ignoring timestamps
    pub full: Option<bool>,
}

// -- Server implementation (tools) --

#[tool_router]
impl NosceServer {
    pub fn new(
        output_dir: PathBuf,
        input_dir: Option<PathBuf>,
        github_owner: Option<String>,
        doc_categories: Vec<String>,
        timezone: Option<String>,
        profiles: Vec<ProfileDef>,
    ) -> Self {
        Self {
            output_dir,
            input_dir,
            github_owner,
            doc_categories,
            timezone,
            profiles,
            tool_router: Self::tool_router(),
            prompt_router: Self::prompt_router(),
        }
    }

    // -- Read-only tools --

    #[tracing::instrument(skip(self), level = "debug")]
    #[tool(
        description = "Get the daily sync report for a specific date. Returns the latest report if no date is provided. Optionally specify a profile for a role-specific view."
    )]
    async fn get_daily_report(
        &self,
        Parameters(params): Parameters<GetDailyReportParams>,
    ) -> Result<CallToolResult, McpError> {
        let base_date = match &params.date {
            Some(date) => Some(date.clone()),
            None => {
                let dates = fs_ops::list_report_dates(&self.output_dir).await;
                dates.into_iter().next()
            }
        };

        let Some(date) = base_date else {
            return Ok(CallToolResult::success(vec![Content::text(
                "No reports found. Run the sync prompt to generate a report.",
            )]));
        };

        // If a profile is requested, try the profile-specific report first
        if let Some(ref profile_id) = params.profile {
            let profile_path = self
                .output_dir
                .join("reports")
                .join(&date)
                .join(format!("{profile_id}.md"));

            if let Ok(content) = fs_ops::read_file(&profile_path).await {
                return Ok(CallToolResult::success(vec![Content::text(content)]));
            }
            // Fall through to base report
        }

        let path = self
            .output_dir
            .join("reports")
            .join(format!("{date}.md"));

        match fs_ops::read_file(&path).await {
            Ok(content) => Ok(CallToolResult::success(vec![Content::text(content)])),
            Err(_) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Report not found for date: {date}",
            ))])),
        }
    }

    #[tracing::instrument(skip(self), level = "debug")]
    #[tool(
        description = "List all available report profiles with their descriptions and focus areas."
    )]
    async fn list_profiles(
        &self,
        #[allow(unused)] Parameters(_params): Parameters<ListProfilesParams>,
    ) -> Result<CallToolResult, McpError> {
        if self.profiles.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                "No profiles configured.",
            )]));
        }

        let output = self
            .profiles
            .iter()
            .map(|p| format!("- **{}** ({}): {}", p.label, p.id, p.description))
            .collect::<Vec<_>>()
            .join("\n");

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Available profiles ({}):\n{output}",
            self.profiles.len(),
        ))]))
    }

    #[tracing::instrument(skip(self), level = "debug")]
    #[tool(description = "List all available daily reports with their dates, most recent first.")]
    async fn list_reports(
        &self,
        Parameters(params): Parameters<ListReportsParams>,
    ) -> Result<CallToolResult, McpError> {
        let limit = params.limit.unwrap_or(30);
        let mut reports = fs_ops::list_report_dates(&self.output_dir).await;
        reports.truncate(limit);

        if reports.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(
                "No reports found. Run the sync prompt to generate reports.",
            )]));
        }

        let output = reports
            .iter()
            .map(|d| format!("- {d}"))
            .collect::<Vec<_>>()
            .join("\n");

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Available reports ({}):\n{output}",
            reports.len(),
        ))]))
    }

    #[tracing::instrument(skip(self), level = "debug")]
    #[tool(
        description = "Get a documentation file by category. Categories: overview, architecture, apis, databases, dependencies."
    )]
    async fn get_doc(
        &self,
        Parameters(params): Parameters<GetDocParams>,
    ) -> Result<CallToolResult, McpError> {
        const VALID: &[&str] = &[
            "overview",
            "architecture",
            "apis",
            "databases",
            "dependencies",
        ];

        if !VALID.contains(&params.category.as_str()) {
            return Ok(CallToolResult::success(vec![Content::text(format!(
                "Unknown category '{}'. Valid categories: {}",
                params.category,
                VALID.join(", ")
            ))]));
        }

        let path = self
            .output_dir
            .join("docs")
            .join(format!("{}.md", params.category));

        match fs_ops::read_file(&path).await {
            Ok(content) => Ok(CallToolResult::success(vec![Content::text(content)])),
            Err(_) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Documentation for '{}' not found. Run the docs prompt to generate it.",
                params.category
            ))])),
        }
    }

    #[tracing::instrument(skip(self), level = "debug")]
    #[tool(
        description = "Search across all generated docs and reports for a query string. Returns matching excerpts with file context."
    )]
    async fn search_docs(
        &self,
        Parameters(params): Parameters<SearchDocsParams>,
    ) -> Result<CallToolResult, McpError> {
        let limit = params.limit.unwrap_or(10);
        let hits = fs_ops::search_all(&self.output_dir, &params.query, limit).await;

        if hits.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(format!(
                "No results found for '{}'.",
                params.query
            ))]));
        }

        let output = hits
            .iter()
            .map(|hit| {
                format!(
                    "**{}** (line {}):\n```\n{}\n```",
                    hit.file, hit.line, hit.context
                )
            })
            .collect::<Vec<_>>()
            .join("\n---\n");

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    #[tracing::instrument(skip(self), level = "debug")]
    #[tool(description = "Get the detailed documentation for a specific submodule by name.")]
    async fn get_submodule_doc(
        &self,
        Parameters(params): Parameters<GetSubmoduleDocParams>,
    ) -> Result<CallToolResult, McpError> {
        let path = self
            .output_dir
            .join("docs")
            .join("submodules")
            .join(format!("{}.md", params.name));

        if let Ok(content) = fs_ops::read_file(&path).await {
            Ok(CallToolResult::success(vec![Content::text(content)]))
        } else {
            let available = fs_ops::list_submodule_names(&self.output_dir).await;
            let hint = if available.is_empty() {
                "No submodule docs found. Run the docs prompt to generate them.".to_owned()
            } else {
                format!(
                    "Submodule '{}' not found. Available: {}",
                    params.name,
                    available.join(", ")
                )
            };
            Ok(CallToolResult::success(vec![Content::text(hint)]))
        }
    }

    #[tracing::instrument(skip(self), level = "debug")]
    #[tool(
        description = "Get changes for a specific submodule across multiple daily reports. Useful for tracking a submodule's evolution over time."
    )]
    async fn get_changelog(
        &self,
        Parameters(params): Parameters<GetChangelogParams>,
    ) -> Result<CallToolResult, McpError> {
        let output_dir = self.output_dir.clone();
        let name = params.name.clone();
        let from = params.from.clone();
        let to = params.to.clone();

        // Run the IO-heavy changelog collection on the blocking threadpool
        let entries = tokio::task::spawn_blocking(move || {
            collect_changelog_sync(&output_dir, &name, from.as_deref(), to.as_deref())
        })
        .await
        .unwrap_or_default();

        if entries.is_empty() {
            return Ok(CallToolResult::success(vec![Content::text(format!(
                "No changelog entries found for submodule '{}'.",
                params.name
            ))]));
        }

        let output = entries
            .iter()
            .map(|(date, section)| format!("# {date}\n\n{section}"))
            .collect::<Vec<_>>()
            .join("\n---\n\n");

        Ok(CallToolResult::success(vec![Content::text(output)]))
    }

    // -- Write tools --

    #[tracing::instrument(skip(self), level = "debug")]
    #[tool(
        description = "Discover git submodules by parsing .gitmodules in the input repository. Returns a JSON array of submodules with name, path, url, and branch."
    )]
    async fn discover_submodules(
        &self,
        Parameters(params): Parameters<DiscoverSubmodulesParams>,
    ) -> Result<CallToolResult, McpError> {
        let input_dir = params
            .input_dir
            .map(|s| PathBuf::from(shellexpand::tilde(&s).into_owned()))
            .or_else(|| self.input_dir.clone());

        let Some(input_dir) = input_dir else {
            return Ok(CallToolResult::success(vec![Content::text(
                "No input directory configured. Provide input_dir parameter or set NOSCE_INPUT_DIR.",
            )]));
        };

        match fs_ops::discover_submodules(&input_dir).await {
            Ok(submodules) => {
                let json = serde_json::to_string_pretty(&submodules)
                    .unwrap_or_else(|e| format!("Failed to serialize: {e}"));
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(err) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Failed to discover submodules: {err}",
            ))])),
        }
    }

    #[tracing::instrument(skip(self), level = "debug")]
    #[tool(
        description = "Get the current sync state with last-synced SHA and timestamp per submodule."
    )]
    async fn get_sync_state(
        &self,
        #[allow(unused)] Parameters(_params): Parameters<GetSyncStateParams>,
    ) -> Result<CallToolResult, McpError> {
        match fs_ops::read_sync_state(&self.output_dir).await {
            Ok(state) => {
                let json = serde_json::to_string_pretty(&state)
                    .unwrap_or_else(|e| format!("Failed to serialize: {e}"));
                Ok(CallToolResult::success(vec![Content::text(json)]))
            }
            Err(err) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Failed to read sync state: {err}",
            ))])),
        }
    }

    #[tracing::instrument(skip(self, params), level = "debug")]
    #[tool(
        description = "Write a daily report markdown file. Creates the base report or a profile-specific summary."
    )]
    async fn write_report(
        &self,
        Parameters(params): Parameters<WriteReportParams>,
    ) -> Result<CallToolResult, McpError> {
        match fs_ops::write_report(
            &self.output_dir,
            &params.date,
            &params.content,
            params.profile.as_deref(),
        )
        .await
        {
            Ok(path) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Report written to {}",
                path.display()
            ))])),
            Err(err) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Failed to write report: {err}",
            ))])),
        }
    }

    #[tracing::instrument(skip(self, params), level = "debug")]
    #[tool(
        description = "Write a documentation markdown file. Creates category docs (overview, architecture, etc.) or submodule docs, optionally with a profile-specific variant."
    )]
    async fn write_doc(
        &self,
        Parameters(params): Parameters<WriteDocParams>,
    ) -> Result<CallToolResult, McpError> {
        match fs_ops::write_doc(
            &self.output_dir,
            &params.name,
            &params.content,
            params.profile.as_deref(),
            params.is_submodule,
        )
        .await
        {
            Ok(path) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Documentation written to {}",
                path.display()
            ))])),
            Err(err) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Failed to write documentation: {err}",
            ))])),
        }
    }

    #[tracing::instrument(skip(self), level = "debug")]
    #[tool(
        description = "Update the sync state with new SHAs for submodules. Merges with existing state."
    )]
    async fn update_sync_state(
        &self,
        Parameters(params): Parameters<UpdateSyncStateParams>,
    ) -> Result<CallToolResult, McpError> {
        // Read current state first
        let mut state = fs_ops::read_sync_state(&self.output_dir)
            .await
            .unwrap_or_default();

        let now = chrono::Utc::now().to_rfc3339();

        for (name, update) in params.submodules {
            state.submodules.insert(
                name,
                fs_ops::SubmoduleState {
                    last_sha: update.last_sha,
                    last_sync: Some(now.clone()),
                    branch: update.branch,
                },
            );
        }

        match fs_ops::write_sync_state(&self.output_dir, &state).await {
            Ok(path) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Sync state updated at {}",
                path.display()
            ))])),
            Err(err) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Failed to update sync state: {err}",
            ))])),
        }
    }

    #[tracing::instrument(skip(self, params), level = "debug")]
    #[tool(
        description = "Write a media file (image/video from PR) and update the manifest. Data should be base64-encoded."
    )]
    async fn write_media(
        &self,
        Parameters(params): Parameters<WriteMediaParams>,
    ) -> Result<CallToolResult, McpError> {
        use base64::Engine;

        let data = match base64::engine::general_purpose::STANDARD.decode(&params.data) {
            Ok(d) => d,
            Err(err) => {
                return Ok(CallToolResult::success(vec![Content::text(format!(
                    "Failed to decode base64 data: {err}",
                ))]));
            }
        };

        match fs_ops::write_media(
            &self.output_dir,
            &params.date,
            &params.filename,
            &data,
            params.manifest_entry,
        )
        .await
        {
            Ok(path) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Media written to {}",
                path.display()
            ))])),
            Err(err) => Ok(CallToolResult::success(vec![Content::text(format!(
                "Failed to write media: {err}",
            ))])),
        }
    }
}

// -- Prompt implementation --

#[prompt_router]
impl NosceServer {
    #[prompt(
        name = "sync",
        description = "Sync all git submodules and generate a daily changelog report with analysis. Collects commits, merged PRs, and diffs, then produces an analytical narrative report."
    )]
    async fn sync_prompt(
        &self,
        Parameters(params): Parameters<SyncPromptParams>,
    ) -> Result<Vec<PromptMessage>, McpError> {
        let date = params
            .date
            .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d").to_string());

        let input_dir = self
            .input_dir
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(not configured — provide via input_dir parameter)".into());

        let content = prompts::build_sync_prompt(
            &input_dir,
            &self.output_dir.display().to_string(),
            self.github_owner.as_deref().unwrap_or("(not configured)"),
            self.timezone.as_deref().unwrap_or("UTC"),
            &date,
            &self.profiles,
        );

        Ok(vec![PromptMessage::new_text(
            PromptMessageRole::User,
            content,
        )])
    }

    #[prompt(
        name = "docs",
        description = "Scan git submodules for architectural information and generate comprehensive documentation about systems, APIs, databases, and service dependencies."
    )]
    async fn docs_prompt(
        &self,
        Parameters(params): Parameters<DocsPromptParams>,
    ) -> Result<Vec<PromptMessage>, McpError> {
        let input_dir = self
            .input_dir
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(not configured — provide via input_dir parameter)".into());

        let content = prompts::build_docs_prompt(
            &input_dir,
            &self.output_dir.display().to_string(),
            self.github_owner.as_deref().unwrap_or("(not configured)"),
            &self.doc_categories,
            params.submodule.as_deref(),
            params.full.unwrap_or(false),
            &self.profiles,
        );

        Ok(vec![PromptMessage::new_text(
            PromptMessageRole::User,
            content,
        )])
    }
}

// -- ServerHandler implementation --

#[tool_handler]
#[prompt_handler(router = self.prompt_router)]
impl ServerHandler for NosceServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::default(),
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_prompts()
                .build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "Nosce MCP server. Provides access to daily sync reports and \
                 architecture documentation generated from git submodule analysis. \
                 Use get_daily_report for changelogs, get_doc for architecture docs, \
                 and search_docs to find specific information. \
                 Use the sync and docs prompts to generate new reports and documentation."
                    .into(),
            ),
        }
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        let mut resources = Vec::new();

        // Latest report (always advertised)
        resources.push(
            RawResource::new("nosce://reports/latest", "Latest Daily Report").no_annotation(),
        );

        // Doc categories — only advertise if the file exists
        for category in &[
            "overview",
            "architecture",
            "apis",
            "databases",
            "dependencies",
        ] {
            let path = self.output_dir.join("docs").join(format!("{category}.md"));
            if fs_ops::path_exists(&path).await {
                let mut raw = RawResource::new(
                    format!("nosce://docs/{category}"),
                    format!("{category} documentation"),
                );
                raw.description = Some(format!("Architecture documentation: {category}"));
                raw.mime_type = Some("text/markdown".into());
                resources.push(raw.no_annotation());
            }
        }

        // Submodule docs
        for name in fs_ops::list_submodule_names(&self.output_dir).await {
            let mut raw = RawResource::new(
                format!("nosce://submodules/{name}"),
                format!("{name} submodule docs"),
            );
            raw.description = Some(format!("Detailed documentation for the {name} submodule"));
            raw.mime_type = Some("text/markdown".into());
            resources.push(raw.no_annotation());
        }

        Ok(ListResourcesResult {
            resources,
            ..Default::default()
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        let uri = request.uri.as_str();

        let path = if uri == "nosce://reports/latest" {
            let Some(p) = fs_ops::find_latest_report(&self.output_dir).await else {
                let content = "No reports available. Run the sync prompt to generate a report.";
                return Ok(ReadResourceResult {
                    contents: vec![ResourceContents::text(content, &request.uri)],
                });
            };
            p
        } else if let Some(category) = uri.strip_prefix("nosce://docs/") {
            self.output_dir.join("docs").join(format!("{category}.md"))
        } else if let Some(name) = uri.strip_prefix("nosce://submodules/") {
            self.output_dir
                .join("docs")
                .join("submodules")
                .join(format!("{name}.md"))
        } else {
            return Err(McpError::resource_not_found(
                format!("Unknown resource: {uri}"),
                None,
            ));
        };

        let content = fs_ops::read_file(&path).await.map_err(|_| {
            McpError::resource_not_found(
                format!("Resource file not found: {}", path.display()),
                None,
            )
        })?;

        Ok(ReadResourceResult {
            contents: vec![ResourceContents::text(content, &request.uri)],
        })
    }
}

// -- Sync helpers for spawn_blocking --

fn collect_changelog_sync(
    output_dir: &Path,
    name: &str,
    from: Option<&str>,
    to: Option<&str>,
) -> Vec<(String, String)> {
    let reports_dir = output_dir.join("reports");
    let mut entries = Vec::new();

    let dir_entries = match std::fs::read_dir(&reports_dir) {
        Ok(e) => e,
        Err(err) => {
            if err.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!("Failed to read reports dir: {err}");
            }
            return entries;
        }
    };

    for entry in dir_entries.flatten() {
        let file_name = entry.file_name();
        let file_str = file_name.to_string_lossy();
        if !file_str.ends_with(".md") {
            continue;
        }
        let date = file_str.trim_end_matches(".md");

        if let Some(f) = from {
            if date < f {
                continue;
            }
        }
        if let Some(t) = to {
            if date > t {
                continue;
            }
        }

        match std::fs::read_to_string(entry.path()) {
            Ok(content) => {
                if let Some(section) = fs_ops::extract_submodule_section(&content, name) {
                    entries.push((date.to_owned(), section));
                }
            }
            Err(err) => {
                tracing::warn!("Failed to read report {}: {err}", entry.path().display());
            }
        }
    }

    entries.sort_by(|a, b| b.0.cmp(&a.0));
    entries
}
