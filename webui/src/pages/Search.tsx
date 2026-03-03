import type { RoutableProps } from 'preact-router';
import { route } from 'preact-router';
import { useEffect, useRef, useState } from 'preact/hooks';

import { apiUrl } from '../api';
import type { SearchData, SearchHit } from '../types';

// eslint-disable-next-line @typescript-eslint/no-empty-object-type
interface SearchProps extends RoutableProps {}

export function Search(_props: SearchProps): preact.JSX.Element {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SearchHit[]>([]);
  const [searching, setSearching] = useState(false);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (!query.trim()) {
      setResults([]);
      return;
    }

    if (timerRef.current) {
      clearTimeout(timerRef.current);
    }

    timerRef.current = setTimeout(() => {
      setSearching(true);
      fetch(apiUrl(`/api/search?q=${encodeURIComponent(query)}`))
        .then((r) => r.json() as Promise<SearchData>)
        .then((data) => {
          setResults(data.results);
          setSearching(false);
        })
        .catch(() => {
          setSearching(false);
        });
    }, 300);

    return () => {
      if (timerRef.current) {
        clearTimeout(timerRef.current);
      }
    };
  }, [query]);

  return (
    <div>
      <h1 class="text-2xl font-bold mb-6 pb-2 border-b border-latte-surface0 dark:border-mocha-surface0">Search</h1>
      <input
        type="search"
        value={query}
        onInput={(e: Event) => {
          const target = e.target as HTMLInputElement;
          setQuery(target.value);
        }}
        placeholder="Search docs and reports..."
        autofocus
        class="w-full px-4 py-3 bg-latte-mantle dark:bg-mocha-mantle border border-latte-surface0 dark:border-mocha-surface0 rounded-lg text-latte-text dark:text-mocha-text placeholder-latte-overlay0 dark:placeholder-mocha-overlay0 focus:border-latte-blue dark:focus:border-mocha-blue focus:outline-none"
      />

      <div class="mt-4 space-y-3">
        {searching && <p class="text-latte-subtext0 dark:text-mocha-subtext0 text-sm">Searching...</p>}

        {!searching && query && results.length === 0 && (
          <p class="text-latte-subtext0 dark:text-mocha-subtext0">No results for &quot;{query}&quot;</p>
        )}

        {results.map((r) => (
          <div
            key={`${r.file}:${String(r.line)}`}
            class="bg-latte-mantle dark:bg-mocha-mantle border border-latte-surface0 dark:border-mocha-surface0 rounded-lg p-4"
          >
            <div class="flex items-center gap-2 mb-2">
              <a
                href={r.url}
                onClick={(e: Event) => {
                  e.preventDefault();
                  route(r.url);
                }}
                class="text-latte-blue dark:text-mocha-blue font-semibold hover:underline"
              >
                {r.file}
              </a>
              <span class="text-xs text-latte-overlay0 dark:text-mocha-overlay0">line {r.line}</span>
            </div>
            <pre class="text-sm bg-latte-crust dark:bg-mocha-crust p-3 rounded overflow-x-auto text-latte-subtext0 dark:text-mocha-subtext0">
              {r.context}
            </pre>
          </div>
        ))}
      </div>
    </div>
  );
}
