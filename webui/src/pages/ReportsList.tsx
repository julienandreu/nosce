import type { RoutableProps } from 'preact-router';
import { route } from 'preact-router';
import { useEffect, useMemo, useState } from 'preact/hooks';

import { apiUrl } from '../api';
import { REPO_COLORS, TAG_COLORS } from '../constants';
import type { ReportEntry, ReportsData } from '../types';

function Tag({ label, color }: { label: string; color: string }): preact.JSX.Element {
  return <span class={`inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium ${color}`}>{label}</span>;
}

// eslint-disable-next-line @typescript-eslint/no-empty-object-type
interface ReportsListProps extends RoutableProps {}

export function ReportsList(_props: ReportsListProps): preact.JSX.Element {
  const [reports, setReports] = useState<ReportEntry[]>([]);
  const [activeTagFilter, setActiveTagFilter] = useState<string | null>(null);
  const [activeRepoFilter, setActiveRepoFilter] = useState<string | null>(null);

  useEffect(() => {
    void fetch(apiUrl('/api/reports'))
      .then((r) => r.json() as Promise<ReportsData>)
      .then((data) => {
        setReports(data.reports);
      });
  }, []);

  const allTags = useMemo(() => {
    const set = new Set<string>();
    reports.forEach((r) => {
      r.tags.forEach((t) => set.add(t));
    });
    return Array.from(set).sort();
  }, [reports]);

  const allRepos = useMemo(() => {
    const set = new Set<string>();
    reports.forEach((r) => {
      r.repos.forEach((rp) => set.add(rp));
    });
    return Array.from(set).sort();
  }, [reports]);

  const filtered = useMemo(() => {
    return reports.filter((r) => {
      if (activeTagFilter && !r.tags.includes(activeTagFilter)) {
        return false;
      }

      if (activeRepoFilter && !r.repos.includes(activeRepoFilter)) {
        return false;
      }

      return true;
    });
  }, [reports, activeTagFilter, activeRepoFilter]);

  const hasFilters = activeTagFilter !== null || activeRepoFilter !== null;

  return (
    <div>
      <div class="flex items-center justify-between mb-6">
        <h1 class="text-2xl font-bold">Reports</h1>
        <span class="text-sm text-latte-overlay0 dark:text-mocha-overlay0">
          {filtered.length} of {reports.length}
        </span>
      </div>

      {/* Filter bar */}
      {(allTags.length > 0 || allRepos.length > 0) && (
        <div class="mb-6 flex flex-wrap items-center gap-2">
          <span class="text-xs font-semibold uppercase tracking-wider text-latte-overlay0 dark:text-mocha-overlay0 mr-1">
            Filter
          </span>

          {allRepos.map((repo) => (
            <button
              key={repo}
              onClick={() => {
                setActiveRepoFilter(activeRepoFilter === repo ? null : repo);
              }}
              class={`px-2.5 py-1 rounded-full text-xs font-medium transition-all cursor-pointer ${
                activeRepoFilter === repo
                  ? 'ring-2 ring-latte-blue dark:ring-mocha-blue ' +
                    (REPO_COLORS[repo] ?? 'bg-latte-surface0 dark:bg-mocha-surface0')
                  : (REPO_COLORS[repo] ?? 'bg-latte-surface0 dark:bg-mocha-surface0')
              }`}
            >
              {repo}
            </button>
          ))}

          {allTags.length > 0 && allRepos.length > 0 && (
            <span class="w-px h-4 bg-latte-surface1 dark:bg-mocha-surface1" />
          )}

          {allTags.map((tag) => (
            <button
              key={tag}
              onClick={() => {
                setActiveTagFilter(activeTagFilter === tag ? null : tag);
              }}
              class={`px-2.5 py-1 rounded-full text-xs font-medium transition-all cursor-pointer ${
                activeTagFilter === tag
                  ? 'ring-2 ring-latte-blue dark:ring-mocha-blue ' +
                    (TAG_COLORS[tag] ?? 'bg-latte-surface0 dark:bg-mocha-surface0')
                  : (TAG_COLORS[tag] ?? 'bg-latte-surface0 dark:bg-mocha-surface0')
              }`}
            >
              {tag}
            </button>
          ))}

          {hasFilters && (
            <button
              onClick={() => {
                setActiveTagFilter(null);
                setActiveRepoFilter(null);
              }}
              class="px-2 py-1 text-xs text-latte-overlay0 dark:text-mocha-overlay0 hover:text-latte-text dark:hover:text-mocha-text cursor-pointer"
            >
              Clear
            </button>
          )}
        </div>
      )}

      {/* Report cards */}
      {filtered.length === 0 ? (
        <div class="text-center py-16 text-latte-subtext0 dark:text-mocha-subtext0">
          {reports.length === 0 ? (
            <>
              <p class="text-lg font-medium mb-2">No reports yet</p>
              <p class="text-sm">
                Run <code class="bg-latte-surface0 dark:bg-mocha-surface0 px-2 py-0.5 rounded">/sync</code> to generate
                reports.
              </p>
            </>
          ) : (
            <p>No reports match the current filters.</p>
          )}
        </div>
      ) : (
        <div class="space-y-3">
          {filtered.map((r) => (
            <a
              key={r.id}
              href={`/reports/${r.id}`}
              onClick={(e: Event) => {
                e.preventDefault();
                route(`/reports/${r.id}`);
              }}
              class="block rounded-xl border border-latte-surface0 dark:border-mocha-surface0 bg-latte-mantle dark:bg-mocha-mantle hover:border-latte-blue/50 dark:hover:border-mocha-blue/50 transition-all overflow-hidden"
            >
              <div class="p-4">
                {/* Header row */}
                <div class="flex items-center gap-3 mb-2">
                  <span class="text-lg font-semibold text-latte-text dark:text-mocha-text">{r.label}</span>
                  <span class="text-sm text-latte-overlay0 dark:text-mocha-overlay0">{r.date_range}</span>
                  {r.commits > 0 && (
                    <span class="ml-auto text-xs font-medium text-latte-overlay1 dark:text-mocha-overlay1 bg-latte-surface0 dark:bg-mocha-surface0 px-2 py-0.5 rounded-full">
                      {r.commits} commits
                    </span>
                  )}
                </div>

                {/* TL;DR */}
                {r.tldr && (
                  <p class="text-sm text-latte-subtext0 dark:text-mocha-subtext0 mb-3 line-clamp-2">{r.tldr}</p>
                )}

                {/* Tags row */}
                <div class="flex flex-wrap gap-1.5">
                  {r.repos.map((repo) => (
                    <Tag
                      key={repo}
                      label={repo}
                      color={REPO_COLORS[repo] ?? 'bg-latte-surface0 dark:bg-mocha-surface0'}
                    />
                  ))}
                  {r.tags.map((tag) => (
                    <Tag key={tag} label={tag} color={TAG_COLORS[tag] ?? 'bg-latte-surface0 dark:bg-mocha-surface0'} />
                  ))}
                </div>
              </div>
            </a>
          ))}
        </div>
      )}
    </div>
  );
}
