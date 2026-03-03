import { route } from 'preact-router';
import { useEffect, useRef, useState } from 'preact/hooks';

import { apiUrl } from '../api';
import type { SearchData, SearchHit } from '../types';

interface CommandPaletteProps {
  open: boolean;
  onClose: () => void;
}

export function CommandPalette({ open, onClose }: CommandPaletteProps): preact.JSX.Element | null {
  const [query, setQuery] = useState('');
  const [results, setResults] = useState<SearchHit[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [searching, setSearching] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Focus input when opening
  useEffect(() => {
    if (open) {
      setQuery('');
      setResults([]);
      setSelectedIndex(0);
      // Small delay to ensure the input is mounted
      requestAnimationFrame(() => {
        inputRef.current?.focus();
      });
    }
  }, [open]);

  // Debounced search
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
          setSelectedIndex(0);
          setSearching(false);
        })
        .catch(() => {
          setSearching(false);
        });
    }, 200);

    return () => {
      if (timerRef.current) {
        clearTimeout(timerRef.current);
      }
    };
  }, [query]);

  const navigate = (url: string): void => {
    onClose();
    route(url);
  };

  const handleKeyDown = (e: KeyboardEvent): void => {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      setSelectedIndex((i) => Math.min(i + 1, results.length - 1));
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      setSelectedIndex((i) => Math.max(i - 1, 0));
    } else if (e.key === 'Enter' && results.length > 0) {
      e.preventDefault();
      const selected = results[selectedIndex];
      if (selected) {
        navigate(selected.url);
      }
    } else if (e.key === 'Escape') {
      e.preventDefault();
      onClose();
    }
  };

  if (!open) {
    return null;
  }

  // Group results by type
  const grouped = new Map<string, SearchHit[]>();
  for (const hit of results) {
    let group = 'Other';
    if (hit.file.startsWith('docs/submodules/')) {
      group = 'Submodules';
    } else if (hit.file.startsWith('docs/')) {
      group = 'Documentation';
    } else if (hit.file.startsWith('reports/')) {
      group = 'Reports';
    }

    const arr = grouped.get(group);
    if (arr) {
      arr.push(hit);
    } else {
      grouped.set(group, [hit]);
    }
  }

  let globalIndex = 0;

  return (
    <div
      class="fixed inset-0 z-50 flex items-start justify-center pt-[15vh]"
      onClick={(e: Event) => {
        if (e.target === e.currentTarget) {
          onClose();
        }
      }}
    >
      {/* Backdrop */}
      <div class="fixed inset-0 bg-black/50 backdrop-blur-sm" onClick={onClose} />

      {/* Palette */}
      <div class="relative w-full max-w-xl bg-latte-base dark:bg-mocha-base border border-latte-surface0 dark:border-mocha-surface0 rounded-xl shadow-2xl overflow-hidden">
        {/* Search input */}
        <div class="flex items-center px-4 border-b border-latte-surface0 dark:border-mocha-surface0">
          <svg
            class="w-5 h-5 text-latte-overlay0 dark:text-mocha-overlay0 shrink-0"
            fill="none"
            stroke="currentColor"
            viewBox="0 0 24 24"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
            />
          </svg>
          <input
            ref={inputRef}
            type="text"
            value={query}
            onInput={(e: Event) => {
              const target = e.target as HTMLInputElement;
              setQuery(target.value);
            }}
            onKeyDown={handleKeyDown}
            placeholder="Search docs, reports, submodules..."
            class="w-full px-3 py-3.5 bg-transparent text-latte-text dark:text-mocha-text placeholder-latte-overlay0 dark:placeholder-mocha-overlay0 focus:outline-none"
          />
          <kbd class="hidden sm:inline-flex items-center px-1.5 py-0.5 text-xs font-medium text-latte-overlay0 dark:text-mocha-overlay0 bg-latte-surface0 dark:bg-mocha-surface0 rounded">
            esc
          </kbd>
        </div>

        {/* Results */}
        <div class="max-h-[50vh] overflow-y-auto">
          {searching && <div class="px-4 py-3 text-sm text-latte-subtext0 dark:text-mocha-subtext0">Searching...</div>}

          {!searching && query && results.length === 0 && (
            <div class="px-4 py-8 text-center text-sm text-latte-subtext0 dark:text-mocha-subtext0">
              No results for &quot;{query}&quot;
            </div>
          )}

          {!searching && results.length > 0 && (
            <div class="py-2">
              {Array.from(grouped.entries()).map(([group, hits]) => (
                <div key={group}>
                  <div class="px-4 py-1.5 text-xs font-semibold uppercase tracking-wider text-latte-overlay0 dark:text-mocha-overlay0">
                    {group}
                  </div>
                  {hits.map((hit) => {
                    const idx = globalIndex++;
                    return (
                      <button
                        key={`${hit.file}:${String(hit.line)}`}
                        onClick={() => {
                          navigate(hit.url);
                        }}
                        class={`w-full text-left px-4 py-2 flex items-center gap-3 cursor-pointer transition-colors ${
                          idx === selectedIndex
                            ? 'bg-latte-blue/10 dark:bg-mocha-blue/10'
                            : 'hover:bg-latte-surface0/50 dark:hover:bg-mocha-surface0/50'
                        }`}
                      >
                        <div class="flex-1 min-w-0">
                          <div class="text-sm font-medium text-latte-text dark:text-mocha-text truncate">
                            {hit.file.replace('.md', '').replace('docs/', '').replace('reports/', '')}
                          </div>
                          {hit.heading && (
                            <div class="text-xs text-latte-overlay1 dark:text-mocha-overlay1 truncate">
                              {hit.heading}
                            </div>
                          )}
                        </div>
                        <svg
                          class="w-4 h-4 text-latte-overlay0 dark:text-mocha-overlay0 shrink-0"
                          fill="none"
                          stroke="currentColor"
                          viewBox="0 0 24 24"
                        >
                          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                        </svg>
                      </button>
                    );
                  })}
                </div>
              ))}
            </div>
          )}

          {!query && (
            <div class="px-4 py-8 text-center text-sm text-latte-subtext0 dark:text-mocha-subtext0">
              Type to search across all docs and reports
            </div>
          )}
        </div>

        {/* Footer */}
        {results.length > 0 && (
          <div class="px-4 py-2 border-t border-latte-surface0 dark:border-mocha-surface0 flex items-center gap-4 text-xs text-latte-overlay0 dark:text-mocha-overlay0">
            <span class="flex items-center gap-1">
              <kbd class="px-1 py-0.5 bg-latte-surface0 dark:bg-mocha-surface0 rounded text-[10px]">&uarr;&darr;</kbd>
              navigate
            </span>
            <span class="flex items-center gap-1">
              <kbd class="px-1 py-0.5 bg-latte-surface0 dark:bg-mocha-surface0 rounded text-[10px]">&crarr;</kbd>
              open
            </span>
            <span class="flex items-center gap-1">
              <kbd class="px-1 py-0.5 bg-latte-surface0 dark:bg-mocha-surface0 rounded text-[10px]">esc</kbd>
              close
            </span>
          </div>
        )}
      </div>
    </div>
  );
}
