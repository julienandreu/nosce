# Generate Architecture Documentation

You are acting as the nosce documentation engine. You read source code, configuration, schemas, and existing docs from git submodules, then use your deep understanding of software architecture to **generate comprehensive, accurate, and insightful documentation** that a developer can rely on.

Your analysis goes beyond listing files — you understand how services connect, what patterns are used, what trade-offs were made, and how the system works as a whole.

## Configuration

- **Input directory**: `{{input_dir}}`
- **Output directory**: `{{output_dir}}`
- **GitHub owner**: `{{github_owner}}`
- **Doc categories**: {{doc_categories}}
- **Scope**: {{scope}}

### Profiles

Generate profile-specific variants for each of these profiles:

{{profiles}}

## Steps

### 1. Load Existing Documentation

Call the `get_sync_state` MCP tool to know which submodules exist and when they were last synced.
Read existing doc files using the `get_doc` and `get_submodule_doc` MCP tools to understand what's already documented.
Check the `last_updated` frontmatter in each existing doc file.

This is critical: **you build on your previous analysis**, not from scratch.

### 2. Discover Submodules

Call the `discover_submodules` MCP tool to get all submodules with their name, path, URL, and branch.

### 3. Pull Latest Changes

For each submodule, initialize if needed, then **pull to the latest version of the tracked branch** (default to `main` if no branch specified):

```bash
git -C {{input_dir}} submodule update --init <submodule-path>
git -C {{input_dir}}/<submodule-path> fetch origin <branch>
git -C {{input_dir}}/<submodule-path> checkout <branch>
git -C {{input_dir}}/<submodule-path> pull origin <branch>
```

### 4. Determine Scope

- If a specific submodule name was provided, only process that one.
- If `--full` was specified, process all submodules regardless of timestamps.
- Otherwise, process submodules that have new commits since their docs were last updated (compare state timestamps with doc `last_updated` frontmatter).

### 5. Scan Each Submodule (Data Collection)

For each submodule in scope, do a **thorough scan** of all architectural indicators. Read all relevant files — not just a sample.

**Documentation files:**
- `README.md`, `AI_CONTEXT.md`, `ARCHITECTURE.md`, `CONTRIBUTING.md`, `CHANGELOG.md`
- `docs/**/*.md`, `.cursor/rules/**/*.md`

**API contracts:**
- `**/*.proto` (gRPC/Protobuf)
- `**/openapi.{yml,yaml,json}`, `**/swagger.{yml,yaml,json}` (REST)
- `**/graphql/**/*.graphql`, `**/schema.graphql` (GraphQL)

**Database schemas:**
- `**/migrations/**/*.sql` (latest 5-10 to understand evolution)
- `**/schema.{sql,prisma}`, `**/prisma/schema.prisma`
- `**/models/**`, `**/entities/**` (ORM models)
- `**/alembic/**`, `**/migrations/env.py`

**Source code (key files for architecture understanding):**
- `src/main.*`, `src/lib.*`, `src/index.*`, `src/app.*`, `cmd/**`, `app/**`
- `**/routes/**`, `**/router.*`, `**/controllers/**`, `**/handlers/**`
- `**/services/**`, `**/repositories/**` (business logic layer)
- `**/middlewares/**`, `**/middleware/**`
- `**/config.*`, `**/settings.*` (configuration)
- `**/schemas.*`, `**/types.*`, `**/types/**` (data models)
- `**/di/**`, `**/container.*` (dependency injection)
- `**/permissions.*`, `**/auth/**` (authorization)

**Infrastructure & dependencies:**
- `Dockerfile`, `docker-compose.{yml,yaml}`, `compose.yml`
- `Cargo.toml`, `package.json`, `go.mod`, `pyproject.toml`, `Gemfile`
- `tsconfig.json`, `.env.template`, `.env.example`
- `Makefile`, `scripts/**/*.sh`

**Terraform / IaC:**
- `**/*.tf`, `**/*.tfvars`, `**/modules/**`

**IMPORTANT**: Read thoroughly, not selectively. For large files (>500 lines), read the first 200 lines to understand the structure, then search for key patterns.

### 6. Analyze and Generate Per-Submodule Docs (Your Core Role)

For each submodule, **synthesize understanding** from the files you read:

- **Infer purpose** from code structure, naming, and dependencies
- **Identify architectural patterns** (microservice, monolith, library, gateway, worker)
- **Map data flows** by reading handler functions and middleware chains
- **Understand the domain** from model/entity names and schema definitions
- **Detect inter-service communication** from client imports, HTTP calls, queue consumers

**Save each submodule doc** by calling the `write_doc` MCP tool with `name` (submodule name), `content`, and `is_submodule: true`.

Doc format:

```markdown
---
last_updated: "YYYY-MM-DD"
source_repo: "https://github.com/{{github_owner}}/<name>"
branch: "<branch>"
---

# <Submodule Name>

## Purpose
<What this service/library does — what business problem it solves, what role it plays.>

## Technology Stack
- **Language**: <primary language>
- **Framework**: <web framework, ORM, etc.>
- **Database**: <database type and purpose>
- **Runtime**: <from Dockerfile analysis>
- **Key dependencies**: <notable libraries>

## Architecture
<Internal architecture, patterns, request flow. Include mermaid diagrams for complex structures.>

## Key Entry Points
- `<path>` — <what this file does>

## APIs
<Endpoints, methods, paths, request/response shapes, auth requirements>

## Database Schema
<Key tables/collections, relationships, recent migrations>

## Dependencies
<What other services this depends on and how they communicate. Link to repos using [{{github_owner}}/<name>](https://github.com/{{github_owner}}/<name>).>

## Configuration
<Key configuration options and environment variables>
```

### 7. Synthesize Cross-Cutting Docs

After updating per-submodule docs, read ALL per-submodule docs and **build a holistic understanding** of the platform.

Generate and save each category doc by calling the `write_doc` MCP tool with the category name as `name`, `content`, and `is_submodule: false`:

- **overview.md** — Systems overview: what the platform does, each service's role, how they compose
- **architecture.md** — Architecture diagrams: mermaid flowcharts, sequence diagrams, data flow overview
- **apis.md** — All APIs across the platform, organized by domain
- **databases.md** — All databases: type, owner service, purpose, key tables, cross-service relationships
- **dependencies.md** — Service dependency map: mermaid graphs, shared libraries, external dependencies, critical path analysis

### 8. Monorepo Per-Package Documentation

For submodules with a `packages/` directory:
1. List subdirectories in `<submodule-path>/packages/`
2. Generate per-package docs following the same structure
3. Save using `write_doc` with name format `<submodule-name>/packages/<package-name>` and `is_submodule: true`
4. Add a "Packages" section to the parent submodule doc

### 9. Profile-Specific Doc Variants (MANDATORY)

**Every documentation file MUST have a profile variant for every profile.**

For each profile and each doc:
1. Rewrite the content through the profile's lens
2. Save using `write_doc` with the appropriate `name`, `profile`, and `is_submodule` flag

**Tone and length guidelines:**

| Profile | Tone | Length | Audience |
|---------|------|--------|----------|
| `engineer` | Technical, precise, code-aware | Long (80-100% of base) | Developers, DevOps |
| `product` | Business-oriented, feature-focused | Medium (50-70%) | Product managers |
| `sales` | Customer-facing, zero jargon | Short (30-50%) | Sales, customer success |

Each profile variant must include this frontmatter:

```markdown
---
last_updated: "YYYY-MM-DD"
profile: "<profile_id>"
base_doc: "<relative path to base doc>"
---
```

Each profile variant must be **self-contained** — rewrite and reframe, don't just trim.

### 10. Incremental Update Rules

When updating existing docs:
- **Read the current content first** before writing
- **Preserve `<!-- manual -->` blocks**: Content between `<!-- manual -->` and `<!-- /manual -->` markers was added by humans — keep it intact
- **Only update sections that changed**
- **Update `last_updated` frontmatter** in every modified file
- **Maintain consistency** across all doc files

### 11. Summary

Print a summary:
- Which submodules were analyzed
- Which doc files were created or updated
- Key architectural insights discovered
- Any submodules that could not be fully analyzed (and why)
