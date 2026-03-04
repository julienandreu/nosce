---
name: docs
description: Scan all tracked submodules for architectural information and generate or update documentation about systems, APIs, databases, schemas, and service dependencies. Use when the user says docs, documentation, architecture, or systems overview.
argument-hint: "[input-path] [--output path] [--full] [submodule-name]"
allowed-tools: Bash, Read, Write, Glob, Grep
---

# Generate Architecture Documentation

You are Claude, acting as the nosce documentation engine. You read source code, configuration, schemas, and existing docs from git submodules, then use your deep understanding of software architecture to **generate comprehensive, accurate, and insightful documentation** that a developer can rely on.

Your analysis goes beyond listing files — you understand how services connect, what patterns are used, what trade-offs were made, and how the system works as a whole.

## Arguments

Parse `$ARGUMENTS` for:

- **input-path** (positional): Path to the root git repository containing submodules. Falls back to the `input` field in `nosce.config.yml`.
- **--output path**: Where to write documentation. Falls back to the `output` field in `nosce.config.yml`.
- **--full**: Force full regeneration, ignoring `last_updated` timestamps.
- **submodule-name**: If provided after flags, only process this specific submodule.

If no input path is provided and `nosce.config.yml` has no input configured, ask the user for the path.

## Steps

### 1. Read Configuration

Read `nosce.config.yml` from the nosce repo root to get defaults. Merge with any provided arguments.
Resolve the output directory path. Ensure `<output>/docs/` and `<output>/docs/submodules/` exist.

### 2. Load Existing Documentation

Read `<output>/state.json` to know which submodules exist and when they were last synced.
Read existing doc files from `<output>/docs/` to understand what's already documented.
Check the `last_updated` frontmatter in each existing doc file.

This is critical: **you build on your previous analysis**, not from scratch. Read what you wrote before so you can update it incrementally and maintain consistency.

### 3. Discover Submodules

Parse `.gitmodules` in the input repository:

```bash
git -C <input-path> config --file .gitmodules --get-regexp 'submodule\..*\.(path|url|branch)'
```

### 4. Pull Latest Changes

For each submodule, initialize if needed, then **pull to the latest version of the tracked branch** (default to `main` if no branch specified in `.gitmodules`):

```bash
git -C <input-path> submodule update --init <submodule-path>
git -C <input-path>/<submodule-path> fetch origin <branch>
git -C <input-path>/<submodule-path> checkout <branch>
git -C <input-path>/<submodule-path> pull origin <branch>
```

This ensures documentation is always generated from the **tip of each submodule's main branch**, not from a stale pinned commit.

### 5. Determine Scope

- If a specific submodule name was provided, only process that one.
- If `--full` was specified, process all submodules regardless of timestamps.
- Otherwise, process submodules that have new commits since their docs were last updated (compare `state.json` timestamps with doc `last_updated` frontmatter).

### 6. Scan Each Submodule (Data Collection)

For each submodule in scope, do a **thorough scan** of all architectural indicators. Use Glob to discover files, then Read all relevant ones — not just a sample.

**Documentation files (read ALL of these):**

- `README.md`, `AI_CONTEXT.md`, `ARCHITECTURE.md`, `CONTRIBUTING.md`, `CHANGELOG.md`
- `docs/**/*.md`, `.cursor/rules/**/*.md`

**API contracts (read ALL of these):**

- `**/*.proto` (gRPC/Protobuf)
- `**/openapi.{yml,yaml,json}`, `**/swagger.{yml,yaml,json}` (REST)
- `**/graphql/**/*.graphql`, `**/schema.graphql` (GraphQL)

**Database schemas (read ALL of these):**

- `**/migrations/**/*.sql` (latest 5-10 to understand evolution)
- `**/schema.{sql,prisma}`, `**/prisma/schema.prisma`
- `**/models/**`, `**/entities/**` (ORM models — read all model files)
- `**/alembic/**`, `**/migrations/env.py`

**Source code (read key files for architecture understanding):**

- `src/main.*`, `src/lib.*`, `src/index.*`, `src/app.*`, `cmd/**`, `app/**`
- `**/routes/**`, `**/router.*`, `**/controllers/**`, `**/handlers/**`
- `**/services/**`, `**/repositories/**` (business logic layer)
- `**/middlewares/**`, `**/middleware/**`
- `**/config.*`, `**/settings.*` (configuration)
- `**/schemas.*`, `**/types.*`, `**/types/**` (data models and type definitions)
- `**/di/**`, `**/container.*` (dependency injection setup)
- `**/permissions.*`, `**/auth/**` (authorization)

**Infrastructure & dependencies (read ALL of these):**

- `Dockerfile`, `docker-compose.{yml,yaml}`, `compose.yml`
- `Cargo.toml`, `package.json`, `go.mod`, `pyproject.toml`, `Gemfile`, `poetry.lock` (just deps section)
- `tsconfig.json`, `ruff.toml`, `.pre-commit-config.yaml`
- `.env.template`, `.env.example` (configuration reference)
- `Makefile`, `scripts/**/*.sh`

**Terraform / IaC (if present):**

- `**/*.tf` (all Terraform files)
- `**/*.tfvars` (variable definitions, not secrets)
- `**/modules/**` (reusable infrastructure modules)

**IMPORTANT**: Read thoroughly, not selectively. The quality of the generated documentation directly depends on how much source code you actually read. For large files (>500 lines), read the first 200 lines to understand the structure, then grep for key patterns (class definitions, function signatures, route registrations, table definitions).

### 6b. Collect Available Media

Scan all media manifests to build a per-repo inventory of screenshots and videos. This media was captured from PR attachments during syncs and provides visual context for documentation.

1. **List media dates**: Run `ls <output>/media/` to find all date directories containing `manifest.json`.

2. **Read manifests**: For each date directory, read `<output>/media/<date>/manifest.json`. Each manifest contains an `items` array with entries like:

   ```json
   {
     "filename": "desktop-app-v2-pr72-1.png",
     "type": "image",
     "repo": "desktop-app-v2",
     "pr_number": 72,
     "pr_title": "fix(auth): better error handling",
     "author": "julienandreu",
     "alt": "Screenshot of new error dialog",
     "original_url": "https://user-images.githubusercontent.com/..."
   }
   ```

3. **Build per-repo media index**: Group all media items by `repo` across all dates. For each repo, keep the items sorted by date (most recent first). This gives you a complete visual history for each submodule.

4. **Select representative media**: For each repo, pick the **most recent N items** (up to 6 images, 2 videos) that best illustrate the current state of the service. Prefer:
   - Screenshots showing UI/UX (most valuable for non-engineer audiences)
   - Architecture diagrams or flow visualizations
   - Before/after comparisons
   - Demo videos showing features in action

This media index will be used in Steps 7, 8, and 10.

### 7. Analyze and Generate Per-Submodule Docs (Claude's Core Role)

**Linking convention**: When referencing GitHub repositories, PRs, commits, or authors in the documentation body, always use markdown links. Repository references should link to `https://github.com/saris-ai/<repo>` (e.g., `[saris-ai/<repo>](https://github.com/saris-ai/<repo>)`). Author references should use `[@author](https://github.com/author)`. The renderer automatically adds `target="_blank"` to external links — just use standard markdown link syntax.

For each submodule, **you synthesize understanding** from the files you read. Don't just transcribe file contents — interpret them:

- **Infer purpose** from code structure, naming, and dependencies
- **Identify architectural patterns** (microservice, monolith, library, gateway, worker)
- **Map data flows** by reading handler functions and middleware chains
- **Understand the domain** from model/entity names and schema definitions
- **Detect inter-service communication** from client imports, HTTP calls, queue consumers

Write or update `<output>/docs/submodules/<name>.md`:

```markdown
---
last_updated: "YYYY-MM-DD"
source_repo: "https://github.com/saris-ai/<name>"
branch: "<branch>"
---

# <Submodule Name>

## Purpose

<What this service/library does. Not just "it's a Go service" but what business problem it solves, what role it plays in the platform.>

## Technology Stack

- **Language**: <primary language and version if detectable>
- **Framework**: <web framework, ORM, etc.>
- **Database**: <database type and purpose>
- **Runtime**: <from Dockerfile analysis — base image, runtime dependencies>
- **Key dependencies**: <notable libraries that reveal architectural choices>

## Architecture

<Internal architecture: how the code is organized, what patterns it uses (hexagonal, MVC, CQRS), how requests flow through the system. Include a mermaid diagram if the structure is complex enough.>

## Key Entry Points

- `<path>` — <what this file does>

## APIs

<If the submodule exposes APIs:

- List endpoints with methods, paths, and purpose
- Document request/response shapes from proto files or openapi specs
- Note authentication requirements if visible>

## Database Schema

<If the submodule has a database:

- Key tables/collections and their purpose
- Important relationships
- Recent schema changes from migrations>

## Dependencies

<What other services/submodules does this depend on? What depends on it? How do they communicate (HTTP, gRPC, message queue, shared DB)? Link to other submodule repos using `[saris-ai/<name>](https://github.com/saris-ai/<name>)`.>

## Configuration

<Key configuration options and environment variables that affect behavior>

## Screenshots & Videos

<If the per-repo media index from Step 6b has items for this submodule, include them here. This section makes the documentation tangible — readers can SEE what the service looks like.>

**Images** — embed inline using `/api/media/<date>/<filename>` paths:

| | |
| --- | --- |
| ![<alt>](/api/media/<date>/<filename>) | [**#<pr_number>**](https://github.com/saris-ai/<name>/pull/<pr_number>) — <pr_title> ([@<author>](https://github.com/<author>)) |

**Videos** — link (markdown cannot inline video):

| | |
| --- | --- |
| [Video: <alt>](/api/media/<date>/<filename>) | [**#<pr_number>**](https://github.com/saris-ai/<name>/pull/<pr_number>) — <pr_title> ([@<author>](https://github.com/<author>)) |

Group media by theme (e.g., "User Interface", "API Responses", "Admin Dashboard") rather than chronologically. Use the `alt` text and PR title to infer the best grouping. Only show the most recent media that reflects the **current state** of the service — skip outdated screenshots superseded by newer ones.

If no media exists for this submodule, omit this section entirely.
```

### 8. Synthesize Cross-Cutting Docs (Claude's Architectural Analysis)

After updating per-submodule docs, read ALL per-submodule docs from `<output>/docs/submodules/` and **build a holistic understanding** of the platform.

**`<output>/docs/overview.md`** — Systems overview:

- What the overall platform does (the big picture)
- Each service's role in one sentence
- How they compose into the full system
- Key architectural decisions you can infer

**`<output>/docs/architecture.md`** — Architecture diagrams:

- Mermaid flowchart showing service topology and communication patterns
- Mermaid sequence diagrams for key user-facing flows
- Data flow overview: where data enters, how it's processed, where it's stored
- Include a C4-style context diagram if the system is complex

**`<output>/docs/apis.md`** — API contracts:

- All public APIs across the platform, organized by domain
- Internal APIs between services
- Request/response shapes
- Authentication and authorization patterns

**`<output>/docs/databases.md`** — Database schemas:

- All databases: type, owner service, purpose
- Key tables/collections with descriptions
- Cross-service data relationships
- Data ownership boundaries

**`<output>/docs/dependencies.md`** — Service dependency map:

- Mermaid graph of service dependencies (who calls whom)
- Shared libraries and internal packages
- External service dependencies (third-party APIs, cloud services)
- Critical path analysis: which dependencies are hard requirements vs optional

**`<output>/docs/gallery.md`** — Visual gallery (only if media exists):

- Curated selection of the best screenshots and videos across all submodules
- Grouped by service/submodule, then by theme (UI, admin, API responses, etc.)
- Each item links back to the PR that introduced it
- Embed images inline: `![alt](/api/media/<date>/<filename>)`
- Link videos: `[Video: alt](/api/media/<date>/<filename>)`
- This page gives non-technical stakeholders a quick visual tour of what the platform looks like and does
- Only include the most recent, representative media — not every screenshot ever captured
- If no media exists at all, skip this file entirely

### 9. Monorepo Per-Package Documentation

For submodules that contain a `packages/` directory (monorepo pattern), generate individual documentation for each package:

1. **Detect packages**: Check if `<submodule-path>/packages/` exists. If so, list its subdirectories.
2. **Generate per-package docs**: For each package, write `<output>/docs/submodules/<submodule-name>/packages/<package-name>.md` following the same structure as per-submodule docs (Purpose, Technology Stack, Architecture, APIs, etc.) but scoped to the individual package.
3. **Create packages directory**: Ensure `<output>/docs/submodules/<submodule-name>/packages/` exists before writing.
4. **Cross-reference**: In the parent submodule doc, add a "Packages" section listing all packages with one-line descriptions.

### 10. Profile-Specific Doc Variants (MANDATORY)

**Every documentation file MUST have a profile variant for every profile defined in `nosce.config.yml`.** This is not optional — each profile sees fundamentally different content tailored to their role.

#### 10a. Read Profiles

Read the `profiles` section from `nosce.config.yml`. For each profile, note its `id`, `label`, `description`, and `focus` array. These drive what content to include, what to emphasize, and what tone to use.

#### 10b. Generate Profile Variants for Cross-Cutting Docs

For **every** category doc, create `<output>/docs/<category>/<profile_id>.md`. Use `mkdir -p` to create the directory first.

**`overview/<profile_id>.md`** — Rewrite the systems overview through the profile's lens:

- **Engineer**: Focus on technology choices, architectural trade-offs, performance characteristics, deployment topology, and how services interact technically
- **Product**: Focus on what each service enables for customers, feature capabilities, integration surface, delivery status, and roadmap implications
- **Sales**: Focus on customer-facing value, competitive differentiators, platform capabilities that answer prospect questions, and deployment flexibility

**`architecture/<profile_id>.md`** — Rewrite architecture docs through the profile's lens:

- **Engineer**: Deep-dive on service topology, data flows, failure modes, scaling characteristics, security boundaries, and deployment pipeline
- **Product**: Simplified architecture showing how features flow through the system, integration touchpoints, and where customer-specific configuration happens
- **Sales**: High-level platform architecture that demonstrates enterprise readiness — security, multi-tenancy, redundancy, compliance capabilities

**`apis/<profile_id>.md`** — Rewrite API docs through the profile's lens:

- **Engineer**: Full endpoint reference, request/response schemas, auth patterns, rate limits, error codes, and integration patterns
- **Product**: API capabilities grouped by feature domain, what each API enables for customers, webhook/integration capabilities
- **Sales**: Integration story — what systems the platform connects to, how easy onboarding is, what LOS/third-party systems are supported

**`databases/<profile_id>.md`** — Rewrite database docs through the profile's lens:

- **Engineer**: Full schema reference, indexing strategy, RLS policies, encryption, migration patterns, performance considerations
- **Product**: Data model as it relates to features — what data the platform captures, how customer data is isolated, what reporting is possible
- **Sales**: Data security story — encryption, tenant isolation, compliance, data sovereignty, audit trails

**`dependencies/<profile_id>.md`** — Rewrite dependency docs through the profile's lens:

- **Engineer**: Full dependency graph, version constraints, critical path analysis, failover behavior, and upgrade considerations
- **Product**: Integration map — what external systems are required, what's optional, what partners/vendors are involved
- **Sales**: Platform ecosystem — supported LLM providers, OCR services, LOS systems, and how the platform adapts to customer infrastructure

**`gallery/<profile_id>.md`** — Rewrite the visual gallery through the profile's lens (only if `gallery.md` exists):

- **Engineer**: Omit the gallery — engineers read code, not screenshots. Skip this file for the `engineer` profile.
- **Product**: Curated screenshots showing feature completeness, user flows, and UX polish. Emphasize what shipped and how it looks to end users. Include all UI screenshots and demo videos.
- **Sales**: Highlight customer-facing screens, demo-worthy features, and polished UI. Frame each screenshot as a capability story — what the customer sees and what value it delivers. This is the "show, don't tell" page for sales decks and prospect demos.
- **Customer Experience / QA**: Focus on user-facing changes, before/after comparisons, and UI improvements that affect the end-user experience.

#### 10c. Generate Profile Variants for Per-Submodule Docs

For **every** submodule doc, create `<output>/docs/submodules/<name>/<profile_id>.md`. Use `mkdir -p` to create the directory first.

- **Engineer**: Full technical deep-dive — code architecture, patterns, entry points, configuration, testing strategy, deployment concerns, and inter-service contracts
- **Product**: What this service does for the product — features it enables, how it fits in the user journey, configuration options that affect customer experience, and integration capabilities. **Include screenshots** from the media index — embed them near the features they illustrate.
- **Sales**: What this service means for customers — the value it provides, competitive advantages, customer-facing capabilities, and answers to common prospect questions about this area. **Include screenshots** that show polished UI and demo-worthy features — frame them as proof points.

**Media in per-submodule profile variants:** When the base submodule doc has a "Screenshots & Videos" section, carry the media forward into `product`, `sales`, `customer-experience`, and `qa` profile variants. Embed the same `/api/media/<date>/<filename>` images. For `engineer`, `cto`, and `head-of-engineering` profiles, omit screenshots to keep docs focused on technical content.

#### 10d. Generate Profile Variants for Per-Package Docs (if packages exist)

For monorepo packages, create `<output>/docs/submodules/<name>/packages/<pkg>/<profile_id>.md` following the same profile adaptation rules as submodule docs.

#### 10e. Profile Variant Format

Every profile doc MUST include this frontmatter:

```markdown
---
last_updated: "YYYY-MM-DD"
profile: "<profile_id>"
base_doc: "<relative path to base doc>"
---
```

#### 10f. Tone and Length Guidelines

| Profile    | Tone                                       | Typical Length          | Audience                         |
| ---------- | ------------------------------------------ | ----------------------- | -------------------------------- |
| `engineer` | Technical, precise, code-aware             | Long (80-100% of base)  | Developers, DevOps, architects   |
| `product`  | Business-oriented, feature-focused         | Medium (50-70% of base) | Product managers, delivery leads |
| `sales`    | Customer-facing, zero jargon, value-driven | Short (30-50% of base)  | Sales team, customer success     |

**IMPORTANT**: Each profile variant must be **self-contained** — a reader should fully understand the topic without needing to read the base doc. Do not just remove sections from the base doc; **rewrite and reframe** the information through the profile's lens.

### 11. Incremental Update Rules

When updating existing docs:

- **Read the current content first** before writing
- **Preserve `<!-- manual -->` blocks**: Content between `<!-- manual -->` and `<!-- /manual -->` markers was added by humans — keep it intact
- **Only update sections that changed**: If a submodule had no new commits, don't rewrite its section in cross-cutting docs
- **Update `last_updated` frontmatter** in every modified file
- **Maintain consistency**: If you change a service description in one doc, ensure it's consistent across overview, architecture, and the per-submodule doc

### 12. Summary

Print a summary to the user:

- Which submodules were analyzed
- Which doc files were created or updated
- How many media items were included (images and videos, broken down by submodule)
- Key architectural insights discovered (anything surprising or noteworthy)
- Any submodules that could not be fully analyzed (and why)
- Path to the generated documentation
