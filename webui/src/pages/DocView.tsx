import type { RoutableProps } from 'preact-router';
import { useCallback, useState } from 'preact/hooks';

import { apiUrl } from '../api';
import { MarkdownContent } from '../components/MarkdownContent';
import { TableOfContents } from '../components/TableOfContents';
import { useProfile } from '../context/ProfileContext';
import type { TocEntry } from '../types';

interface DocViewProps extends RoutableProps {
  category?: string;
}

export function DocView({ category }: DocViewProps): preact.JSX.Element | null {
  const { profileId, currentProfile } = useProfile();
  const [toc, setToc] = useState<TocEntry[]>([]);
  const [servedProfile, setServedProfile] = useState<string | null>(null);

  const handleToc = useCallback((entries: TocEntry[]) => {
    setToc(entries);
  }, []);

  if (!category) {
    return null;
  }

  const url = apiUrl(`/api/docs/${category}?profile=${encodeURIComponent(profileId)}`);

  return (
    <>
      <div class="flex-1 min-w-0">
        {currentProfile && (
          <div class="mb-4 inline-flex items-center gap-2 px-3 py-1.5 bg-latte-surface0 dark:bg-mocha-surface0 rounded-full text-sm text-latte-subtext1 dark:text-mocha-subtext1">
            <span class="font-medium">{currentProfile.label}</span>
            {servedProfile ? (
              <span class="text-latte-overlay0 dark:text-mocha-overlay0">view</span>
            ) : (
              <span class="text-latte-overlay0 dark:text-mocha-overlay0">
                — base content (no {currentProfile.label.toLowerCase()} variant)
              </span>
            )}
          </div>
        )}
        <MarkdownContent key={url} url={url} onToc={handleToc} onProfile={setServedProfile} />
      </div>
      <aside class="hidden lg:block w-56 shrink-0 sticky top-8 self-start ml-8 max-h-[calc(100vh-4rem)] overflow-y-auto">
        <TableOfContents toc={toc} />
      </aside>
    </>
  );
}
