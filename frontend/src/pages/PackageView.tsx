import type { RoutableProps } from 'preact-router';
import { route } from 'preact-router';

import { apiUrl } from '../api';
import { MarkdownContent } from '../components/MarkdownContent';
import { useProfile } from '../context/ProfileContext';

interface PackageViewProps extends RoutableProps {
  name?: string;
  pkg?: string;
}

export function PackageView({ name, pkg }: PackageViewProps): preact.JSX.Element | null {
  const { profileId, currentProfile } = useProfile();

  if (!name || !pkg) {
    return null;
  }

  const url = apiUrl(`/api/submodules/${name}/packages/${pkg}?profile=${encodeURIComponent(profileId)}`);

  return (
    <div>
      {/* Breadcrumb */}
      <div class="mb-4 flex items-center gap-2 text-sm text-latte-subtext0 dark:text-mocha-subtext0">
        <a
          href={`/submodules/${name}`}
          onClick={(e: Event) => {
            e.preventDefault();
            route(`/submodules/${name}`);
          }}
          class="text-latte-blue dark:text-mocha-blue hover:underline"
        >
          {name}
        </a>
        <span>/</span>
        <span class="text-latte-text dark:text-mocha-text font-medium">{pkg}</span>
      </div>

      {currentProfile && (
        <div class="mb-4 inline-flex items-center gap-2 px-3 py-1.5 bg-latte-surface0 dark:bg-mocha-surface0 rounded-full text-sm text-latte-subtext1 dark:text-mocha-subtext1">
          <span class="font-medium">{currentProfile.label}</span>
          <span class="text-latte-overlay0 dark:text-mocha-overlay0">view</span>
        </div>
      )}

      <MarkdownContent key={url} url={url} />
    </div>
  );
}
