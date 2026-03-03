import type { ComponentChildren } from 'preact';
import type { RoutableProps } from 'preact-router';
import { route } from 'preact-router';
import { useEffect, useState } from 'preact/hooks';

import { apiUrl } from '../api';
import { DOC_ICONS, TAG_COLORS } from '../constants';
import { useProfile } from '../context/ProfileContext';
import type { NavData, ReportEntry, ReportsData } from '../types';

interface CardProps {
  children: ComponentChildren;
  className?: string;
}

function Card({ children, className }: CardProps): preact.JSX.Element {
  return (
    <div
      class={`bg-latte-mantle dark:bg-mocha-mantle border border-latte-surface0 dark:border-mocha-surface0 rounded-xl p-5 ${className ?? ''}`}
    >
      {children}
    </div>
  );
}

function MetricCard({ label, value, icon }: { label: string; value: string; icon: string }): preact.JSX.Element {
  return (
    <Card>
      <div class="flex items-center gap-3">
        <div class="w-10 h-10 rounded-lg bg-latte-blue/10 dark:bg-mocha-blue/10 flex items-center justify-center">
          <svg
            class="w-5 h-5 text-latte-blue dark:text-mocha-blue"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d={icon} />
          </svg>
        </div>
        <div>
          <div class="text-2xl font-bold text-latte-text dark:text-mocha-text">{value}</div>
          <div class="text-xs text-latte-subtext0 dark:text-mocha-subtext0">{label}</div>
        </div>
      </div>
    </Card>
  );
}

// eslint-disable-next-line @typescript-eslint/no-empty-object-type
interface HomeProps extends RoutableProps {}

export function Home(_props: HomeProps): preact.JSX.Element | null {
  const [data, setData] = useState<NavData | null>(null);
  const [latestReport, setLatestReport] = useState<ReportEntry | null>(null);
  const { currentProfile } = useProfile();

  useEffect(() => {
    void fetch(apiUrl('/api/nav'))
      .then((r) => r.json() as Promise<NavData>)
      .then(setData);

    void fetch(apiUrl('/api/reports'))
      .then((r) => r.json() as Promise<ReportsData>)
      .then((d) => {
        if (d.reports.length > 0 && d.reports[0]) {
          setLatestReport(d.reports[0]);
        }
      });
  }, []);

  if (!data) {
    return null;
  }

  const totalCommits = latestReport?.commits ?? 0;
  const activeRepos = latestReport?.repos.length ?? 0;
  const reportCount = data.reports.length;

  return (
    <div class="max-w-3xl mx-auto">
      {/* Profile greeting */}
      {currentProfile && (
        <div class="mb-8 flex items-center gap-3">
          <div class="w-10 h-10 rounded-full bg-latte-blue/10 dark:bg-mocha-blue/10 flex items-center justify-center text-latte-blue dark:text-mocha-blue font-bold text-lg">
            {currentProfile.label.charAt(0)}
          </div>
          <div>
            <h1 class="text-xl font-bold text-latte-text dark:text-mocha-text">{currentProfile.label} Dashboard</h1>
            <p class="text-sm text-latte-subtext0 dark:text-mocha-subtext0">{currentProfile.description}</p>
          </div>
        </div>
      )}

      {/* Metric cards */}
      {latestReport && (
        <div class="grid grid-cols-3 gap-4 mb-6">
          <MetricCard
            label="Commits (latest)"
            value={String(totalCommits)}
            icon="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2"
          />
          <MetricCard
            label="Active repos"
            value={String(activeRepos)}
            icon="M3 7v10a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-6l-2-2H5a2 2 0 00-2 2z"
          />
          <MetricCard
            label="Reports"
            value={String(reportCount)}
            icon="M9 12h6m-6 4h6m2 5H7a2 2 0 01-2-2V5a2 2 0 012-2h5.586a1 1 0 01.707.293l5.414 5.414a1 1 0 01.293.707V19a2 2 0 01-2 2z"
          />
        </div>
      )}

      {/* Latest report */}
      {latestReport && (
        <a
          href={`/reports/${latestReport.id}`}
          onClick={(e: Event) => {
            e.preventDefault();
            route(`/reports/${latestReport.id}`);
          }}
          class="block mb-6 rounded-xl border border-latte-surface0 dark:border-mocha-surface0 bg-latte-mantle dark:bg-mocha-mantle hover:border-latte-blue/50 dark:hover:border-mocha-blue/50 transition-all p-5"
        >
          <div class="flex items-center gap-3 mb-2">
            <h2 class="text-lg font-semibold text-latte-text dark:text-mocha-text">Latest: {latestReport.label}</h2>
            <span class="text-sm text-latte-overlay0 dark:text-mocha-overlay0">{latestReport.date_range}</span>
          </div>
          {latestReport.tldr && (
            <p class="text-sm text-latte-subtext0 dark:text-mocha-subtext0 mb-3 line-clamp-2">{latestReport.tldr}</p>
          )}
          {latestReport.tags.length > 0 && (
            <div class="flex flex-wrap gap-1.5">
              {latestReport.tags.map((tag) => (
                <span
                  key={tag}
                  class={`inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium ${TAG_COLORS[tag] ?? 'bg-latte-surface0 dark:bg-mocha-surface0'}`}
                >
                  {tag}
                </span>
              ))}
            </div>
          )}
        </a>
      )}

      {/* Two-column grid: docs + submodules */}
      <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
        {/* Documentation cards */}
        {data.docs.length > 0 && (
          <div>
            <h2 class="text-sm font-semibold uppercase tracking-wider text-latte-overlay0 dark:text-mocha-overlay0 mb-3">
              Documentation
            </h2>
            <div class="space-y-2">
              {data.docs.map((cat) => (
                <a
                  key={cat}
                  href={`/docs/${cat}`}
                  onClick={(e: Event) => {
                    e.preventDefault();
                    route(`/docs/${cat}`);
                  }}
                  class="flex items-center gap-3 p-3 rounded-lg border border-latte-surface0 dark:border-mocha-surface0 bg-latte-mantle dark:bg-mocha-mantle hover:border-latte-blue/50 dark:hover:border-mocha-blue/50 transition-all"
                >
                  <svg
                    class="w-5 h-5 text-latte-blue dark:text-mocha-blue shrink-0"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="1.5"
                      d={DOC_ICONS[cat] ?? DOC_ICONS['overview'] ?? ''}
                    />
                  </svg>
                  <span class="font-medium text-sm text-latte-text dark:text-mocha-text capitalize">{cat}</span>
                </a>
              ))}
            </div>
          </div>
        )}

        {/* Submodule cards */}
        {data.submodules.length > 0 && (
          <div>
            <h2 class="text-sm font-semibold uppercase tracking-wider text-latte-overlay0 dark:text-mocha-overlay0 mb-3">
              Submodules
            </h2>
            <div class="space-y-2">
              {data.submodules.map((sub) => (
                <a
                  key={sub.name}
                  href={`/submodules/${sub.name}`}
                  onClick={(e: Event) => {
                    e.preventDefault();
                    route(`/submodules/${sub.name}`);
                  }}
                  class="flex items-center gap-3 p-3 rounded-lg border border-latte-surface0 dark:border-mocha-surface0 bg-latte-mantle dark:bg-mocha-mantle hover:border-latte-blue/50 dark:hover:border-mocha-blue/50 transition-all"
                >
                  <svg
                    class="w-5 h-5 text-latte-teal dark:text-mocha-teal shrink-0"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="1.5"
                      d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"
                    />
                  </svg>
                  <span class="font-medium text-sm text-latte-text dark:text-mocha-text">{sub.name}</span>
                  {sub.packages.length > 0 && (
                    <span class="ml-auto text-xs px-2 py-0.5 rounded-full bg-latte-surface0 dark:bg-mocha-surface0 text-latte-overlay1 dark:text-mocha-overlay1">
                      {sub.packages.length} pkg{sub.packages.length > 1 ? 's' : ''}
                    </span>
                  )}
                </a>
              ))}
            </div>
          </div>
        )}
      </div>

      {/* Empty state */}
      {!data.latest_report && data.docs.length === 0 && (
        <div class="text-center mt-16 text-latte-subtext0 dark:text-mocha-subtext0">
          <h2 class="text-xl font-semibold mb-2">No documentation yet</h2>
          <p>
            Run <code class="bg-latte-surface0 dark:bg-mocha-surface0 px-2 py-0.5 rounded">/sync</code> and{' '}
            <code class="bg-latte-surface0 dark:bg-mocha-surface0 px-2 py-0.5 rounded">/docs</code> to generate reports
            and documentation.
          </p>
        </div>
      )}
    </div>
  );
}
