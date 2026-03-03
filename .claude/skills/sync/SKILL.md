---
name: sync
description: Sync all git submodules from a platform repository, identify new commits and merged PRs since last sync, and generate a daily markdown report. Use when the user says sync, daily report, changelog, or what changed.
argument-hint: "[input-path] [--output path] [--date YYYY-MM-DD]"
allowed-tools: Bash, Read, Write, Glob, Grep
---

# Sync Submodules and Generate Daily Report

You are Claude, acting as the nosce sync engine. You collect raw data from git repositories (commits, PRs, diffs) and then **analyze, correlate, and summarize** it using your understanding of software engineering. Your analysis is the core value — not just listing commits, but explaining what they mean.

## Arguments

Parse `$ARGUMENTS` for:

- **input-path** (positional): Path to the root git repository containing submodules. Falls back to the `input` field in `nosce.config.yml`.
- **--output path**: Where to write reports and state. Falls back to the `output` field in `nosce.config.yml`.
- **--date YYYY-MM-DD**: Override the report date (default: today).

If no input path is provided and `nosce.config.yml` has no input configured, ask the user for the path.

## Steps

### 1. Read Configuration

Read `nosce.config.yml` from the nosce repo root to get defaults. Merge with any provided arguments.
Resolve the output directory path (expand `~` if needed). Ensure it exists (create with `mkdir -p` if not).

### 2. Load Previous State

Read `<output>/state.json` if it exists. This contains the last-synced SHA and timestamp per submodule.
If the file doesn't exist, this is a first run — create an empty state object.

```json
{
  "submodules": {
    "<name>": {
      "last_sha": "<commit-sha>",
      "last_sync": "<ISO-8601 timestamp>",
      "branch": "<branch-name>"
    }
  }
}
```

### 3. Discover Submodules

Parse the `.gitmodules` file in the input repository to discover all submodules:

```bash
git -C <input-path> config --file .gitmodules --get-regexp 'submodule\..*\.(path|url|branch)'
```

Build a list of submodules with their path, URL, and branch (default to `main` if no branch specified).

### 4. Pull Latest Changes

For each submodule, initialize if needed, then **pull to the latest version of the tracked branch**:

```bash
git -C <input-path> submodule update --init <submodule-path>
git -C <input-path>/<submodule-path> fetch origin <branch>
git -C <input-path>/<submodule-path> checkout <branch>
git -C <input-path>/<submodule-path> pull origin <branch>
git -C <input-path>/<submodule-path> rev-parse HEAD
```

This ensures submodules are always at the **tip of their main branch**, not pinned to an old commit.

### 5. Collect Raw Data

For each submodule, gather the raw materials you will analyze:

**Commits** (if previous SHA exists in state):

```bash
git -C <input-path>/<submodule-path> log --format='%H|%an|%ae|%aI|%s' <last_sha>..origin/<branch>
```

If no previous SHA (first run), limit to last 24 hours:

```bash
git -C <input-path>/<submodule-path> log --format='%H|%an|%ae|%aI|%s' --since="24 hours ago" origin/<branch>
```

**Merged PRs** (extract GitHub owner/repo from URL):

```bash
gh pr list -R <owner>/<repo> --state merged --json number,title,author,mergedAt,additions,deletions,headRefName,body --search "merged:>=<last-sync-date>" --limit 100
```

**Diff stats** (to understand scope of changes):

```bash
git -C <input-path>/<submodule-path> diff --stat <last_sha>..origin/<branch>
```

If `gh` fails (rate limit, auth issues), note it and continue without PR data.

### 5b. Extract and Download PR Media

For each merged PR that has a `body` field, extract and download any screenshots or videos:

1. **Parse media URLs** from the PR body markdown:
   - Image patterns: `![...](url)` or `<img src="url">`
   - Video patterns: `![...](url.mp4)`, `![...](url.mov)`, `<video src="url">`, or GitHub user-attachments video blocks
   - Only keep files with extensions: `.png`, `.jpg`, `.jpeg`, `.gif`, `.webp`, `.mp4`, `.mov`, `.webm`
   - Skip badge images (URLs containing `shields.io`, `img.shields.io`, `badge`)

2. **Download media** to `<output>/media/<date>/`:

   ```bash
   mkdir -p <output>/media/<date>
   curl -sL -o <output>/media/<date>/<repo>-pr<number>-<index>.<ext> "<url>"
   ```

   Filename format: `<repo>-pr<number>-<index>.<ext>` (e.g., `desktop-app-v2-pr72-1.png`)

3. **Build manifest** at `<output>/media/<date>/manifest.json`:

   ```json
   {
     "date": "YYYY-MM-DD",
     "items": [
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
     ]
   }
   ```

   The `type` field is `"image"` for `.png`, `.jpg`, `.jpeg`, `.gif`, `.webp` and `"video"` for `.mp4`, `.mov`, `.webm`.

4. **Skip on failure** — If a URL 404s or download fails, log a warning and continue. Never block the sync on media failures.

### 5c. Load Media Manifest

Read `<output>/media/<date>/manifest.json` if it exists. This tells you which submodules have screenshots/videos from their PRs. You will need this in Step 6 to embed media in the report. If the file doesn't exist, there is no media for this date.

### 6. Analyze and Generate Report (Claude's Core Role)

This is where you, Claude, add value beyond raw data. Create `<output>/reports/YYYY-MM-DD.md`:

**For each submodule with changes, you must:**

1. **Group related commits** — Don't just list commits chronologically. Cluster them by theme: feature work, bug fixes, refactoring, CI/CD changes, dependency updates.

2. **Identify the narrative** — What story do these changes tell? Is a new feature being built across multiple commits? Was a bug found and fixed? Is there a migration underway?

3. **Assess impact** — Based on the files changed and the diff stats, which changes are high-impact (touching core logic, APIs, schemas) vs low-impact (docs, formatting, tests)?

4. **Correlate PRs with commits** — Match merged PRs to their commits. Use PR descriptions (the `body` field) to understand intent.

5. **Highlight breaking changes** — If you see schema migrations, API changes, interface modifications, or dependency major version bumps, flag them prominently.

6. **Credit contributors** — Note who did what, especially if multiple people are working on the same submodule.

**Report format:**

```markdown
# Nosce Daily Report — YYYY-MM-DD

> Generated at <ISO-8601 timestamp> by Claude

## Summary

- **N** submodules analyzed
- **N** new commits across all submodules
- **N** PRs merged
- Key highlights: <1-2 sentence executive summary of the most important changes>

---

## <submodule-name>

**Branch**: `<branch>` | **Repo**: [saris-ai/<submodule-name>](https://github.com/saris-ai/<submodule-name>)

### Changes Overview

<Your analytical summary: 2-5 sentences explaining what changed, why it matters, and any risks or notable patterns. This is the most valuable section — write it for a developer who needs to understand what happened without reading every commit.>

### Commits (N new)

| SHA                                                                         | Author      | Message        | Date       |
| --------------------------------------------------------------------------- | ----------- | -------------- | ---------- |
| [`abc1234`](https://github.com/saris-ai/<submodule-name>/commit/<full-sha>) | Author Name | commit message | YYYY-MM-DD |

### Merged PRs (N)

- [**#42**](https://github.com/saris-ai/<submodule-name>/pull/42) — PR title ([@author](https://github.com/author), merged YYYY-MM-DD) — +120 -45
  - Branch: `feature/thing`
  - <1-sentence summary from PR body if available>

### Impact Assessment

- **Risk level**: Low/Medium/High
- **Areas affected**: <list of architectural areas touched>
- **Action items**: <any follow-up needed, e.g., "review migration before deploying">

### Screenshots & Videos

**MANDATORY when media exists.** Before writing each submodule section, read `<output>/media/<date>/manifest.json` (created in Step 5b). Filter the `items` array for entries matching `"repo": "<submodule-name>"`. If any items match, you MUST include this section — do not skip it.

For each matching media item, render a row in the table below. Use the item's `filename`, `pr_number`, `pr_title`, `author`, and `alt` fields from the manifest.

**Images** — embed inline with `![alt](/api/media/<date>/<filename>)`:

|                                            |                                                                                                                                           |
| ------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------- |
| ![<alt>](/api/media/YYYY-MM-DD/<filename>) | [**#<pr_number>**](https://github.com/saris-ai/<submodule-name>/pull/<pr_number>) — <pr_title> ([@<author>](https://github.com/<author>)) |

**Videos** — link instead of embedding (markdown can't inline video):

|                                                  |                                                                                                                                           |
| ------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------- |
| [Video: <alt>](/api/media/YYYY-MM-DD/<filename>) | [**#<pr_number>**](https://github.com/saris-ai/<submodule-name>/pull/<pr_number>) — <pr_title> ([@<author>](https://github.com/<author>)) |

Group rows by PR number. Include ALL media items from the manifest for this repo — do not cherry-pick.

---

## No Changes

- submodule-a (already up to date)
- submodule-b (already up to date)
```

### 6b. Generate Profile-Specific Summaries

After generating the base report, create focused summaries for each audience.

Read the `profiles` section from `nosce.config.yml` in the nosce repo root. For each profile:

1. Create directory `<output>/reports/YYYY-MM-DD/` if it doesn't exist (use `mkdir -p`)
2. Re-read the base report you just wrote at `<output>/reports/YYYY-MM-DD.md`
3. Generate `<output>/reports/YYYY-MM-DD/<profile-id>.md` with a focused summary

**Profile report format:**

```markdown
---
profile: <profile-id>
base_report: YYYY-MM-DD
generated_at: "<ISO-8601>"
---

# Daily Report — YYYY-MM-DD (<Profile Label> View)

## TL;DR

<2-4 bullet points, written for this specific audience>

## Key Changes

<Re-analyze the base report through this profile's lens.
Only include information relevant to the profile's focus areas.
Use the tone and detail level appropriate for the audience.>

## Action Items

<Specific actionable items relevant to this profile>
```

**Profile lens guidelines:**

| Profile               | Emphasis                                                | Tone                         | Length            |
| --------------------- | ------------------------------------------------------- | ---------------------------- | ----------------- |
| `engineer`            | Commit diffs, breaking changes, tech debt, architecture | Technical, detailed          | Long              |
| `pm`                  | Feature progress, user stories, blockers, timeline      | Business-oriented            | Medium            |
| `head-of-engineering` | Velocity, cross-team deps, risk, staffing               | Strategic-technical          | Medium            |
| `cto`                 | 3-bullet executive summary, platform health, key risks  | Executive, concise           | Short (~10 lines) |
| `product`             | Feature completeness, roadmap alignment, user value     | Product-oriented             | Medium            |
| `sales`               | Customer-facing features, competitive advantages, demos | Customer-facing, zero jargon | Short             |
| `customer-experience` | Bug fixes, UX changes, breaking user-facing changes     | User-centric                 | Medium            |
| `technical-pm`        | Delivery status, deps, integration risks, sprint health | Project management           | Medium            |
| `qa`                  | Risk areas needing testing, regression, deploy safety   | Testing-focused              | Medium            |

**Media in profiles:** When the base report includes Screenshots & Videos sections, include relevant screenshots in profile summaries too — especially for `product`, `customer-experience`, `sales`, and `qa` profiles. Embed the same `/api/media/<date>/<filename>` images. For `cto` and `head-of-engineering`, omit screenshots to keep summaries concise.

**Important:** Each profile summary should be self-contained — a reader should understand the day's changes without needing to read the base report. Do not just copy sections from the base report; rewrite and reframe the information through the profile's lens.

### 7. Update State

Write the updated state to `<output>/state.json` with new SHAs and current timestamp for each submodule.

### 8. Summary

Print a concise summary to the user:

- How many submodules were analyzed
- How many had changes
- Total commits and PRs found
- Key highlights (repeat the executive summary)
- Path to the generated report
