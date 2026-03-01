import { useEffect, useState } from 'preact/hooks';

import type { TocEntry } from '../types';

interface TableOfContentsProps {
  toc: TocEntry[];
}

export function TableOfContents({ toc }: TableOfContentsProps): preact.JSX.Element | null {
  const [activeId, setActiveId] = useState<string>('');

  useEffect(() => {
    if (toc.length === 0) return;

    const headingElements = toc
      .map((entry) => document.getElementById(entry.id))
      .filter((el): el is HTMLElement => el !== null);

    if (headingElements.length === 0) return;

    const observer = new IntersectionObserver(
      (entries) => {
        // Find the first intersecting entry from top
        const visible = entries
          .filter((e) => e.isIntersecting)
          .sort((a, b) => a.boundingClientRect.top - b.boundingClientRect.top);

        if (visible.length > 0 && visible[0]) {
          setActiveId(visible[0].target.id);
        }
      },
      {
        rootMargin: '-80px 0px -60% 0px',
        threshold: 0,
      },
    );

    headingElements.forEach((el) => observer.observe(el));

    return () => observer.disconnect();
  }, [toc]);

  if (toc.length === 0) return null;

  const scrollTo = (id: string): void => {
    const el = document.getElementById(id);
    if (el) {
      el.scrollIntoView({ behavior: 'smooth', block: 'start' });
      setActiveId(id);
    }
  };

  return (
    <nav class="space-y-1">
      <h4 class="text-xs font-semibold uppercase tracking-wider text-latte-overlay0 dark:text-mocha-overlay0 mb-3">
        On this page
      </h4>
      {toc.map((entry) => (
        <a
          key={entry.id}
          href={`#${entry.id}`}
          onClick={(e: Event) => {
            e.preventDefault();
            scrollTo(entry.id);
          }}
          class={`block text-sm py-0.5 transition-colors border-l-2 ${
            entry.level === 3 ? 'pl-5' : 'pl-3'
          } ${
            activeId === entry.id
              ? 'border-latte-blue dark:border-mocha-blue text-latte-blue dark:text-mocha-blue font-medium'
              : 'border-transparent text-latte-subtext0 dark:text-mocha-subtext0 hover:text-latte-text dark:hover:text-mocha-text'
          }`}
        >
          {entry.text}
        </a>
      ))}
    </nav>
  );
}
