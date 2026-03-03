import type { ComponentChildren } from 'preact';
import { route } from 'preact-router';
import { useEffect, useState } from 'preact/hooks';

import type { NavData, SubmoduleNav } from '../types';
import { ProfileSelector } from './ProfileSelector';

function useCurrentPath(): string {
  const [path, setPath] = useState(window.location.pathname);

  useEffect(() => {
    const handler = (): void => {
      setPath(window.location.pathname);
    };
    window.addEventListener('popstate', handler);

    // Intercept pushState/replaceState
    const origPush = history.pushState.bind(history);
    const origReplace = history.replaceState.bind(history);
    history.pushState = (...args) => {
      origPush(...args);
      handler();
    };
    history.replaceState = (...args) => {
      origReplace(...args);
      handler();
    };

    return () => {
      window.removeEventListener('popstate', handler);
      history.pushState = origPush;
      history.replaceState = origReplace;
    };
  }, []);

  return path;
}

interface NavLinkProps {
  href: string;
  active: boolean;
  children: ComponentChildren;
}

function NavLink({ href, active, children }: NavLinkProps): preact.JSX.Element {
  const navigate = (e: Event): void => {
    e.preventDefault();
    route(href);
  };

  return (
    <a
      href={href}
      onClick={navigate}
      class={`block px-3 py-1.5 rounded-md text-sm transition-colors ${
        active
          ? 'bg-latte-blue/10 dark:bg-mocha-blue/10 text-latte-blue dark:text-mocha-blue font-medium'
          : 'text-latte-subtext1 dark:text-mocha-subtext1 hover:bg-latte-surface0 dark:hover:bg-mocha-surface0 hover:text-latte-blue dark:hover:text-mocha-blue'
      }`}
    >
      {children}
    </a>
  );
}

interface SectionTitleProps {
  children: ComponentChildren;
}

function SectionTitle({ children }: SectionTitleProps): preact.JSX.Element {
  return (
    <h3 class="text-xs font-semibold uppercase tracking-wider text-latte-overlay0 dark:text-mocha-overlay0 mt-6 mb-2 px-3">
      {children}
    </h3>
  );
}

function SubmoduleTree({ sub, currentPath }: { sub: SubmoduleNav; currentPath: string }): preact.JSX.Element {
  const isSubActive = currentPath === `/submodules/${sub.name}` || currentPath.startsWith(`/submodules/${sub.name}/`);
  const [expanded, setExpanded] = useState(isSubActive);

  const hasPackages = sub.packages.length > 0;

  return (
    <div>
      <div class="flex items-center">
        {hasPackages && (
          <button
            onClick={() => {
              setExpanded(!expanded);
            }}
            class="w-5 h-5 flex items-center justify-center text-latte-overlay0 dark:text-mocha-overlay0 hover:text-latte-text dark:hover:text-mocha-text cursor-pointer shrink-0"
          >
            <svg
              class={`w-3 h-3 transition-transform ${expanded ? 'rotate-90' : ''}`}
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
            </svg>
          </button>
        )}
        <div class={hasPackages ? '' : 'pl-5'} style={{ flex: '1', minWidth: 0 }}>
          <NavLink href={`/submodules/${sub.name}`} active={currentPath === `/submodules/${sub.name}`}>
            {sub.name}
          </NavLink>
        </div>
      </div>

      {hasPackages && expanded && (
        <div class="ml-5 pl-3 border-l border-latte-surface0 dark:border-mocha-surface0">
          {sub.packages.map((pkg) => (
            <NavLink
              key={pkg}
              href={`/submodules/${sub.name}/packages/${pkg}`}
              active={currentPath === `/submodules/${sub.name}/packages/${pkg}`}
            >
              {pkg}
            </NavLink>
          ))}
        </div>
      )}
    </div>
  );
}

const DOC_LABELS: Record<string, string> = {
  overview: 'Overview',
  architecture: 'Architecture',
  apis: 'APIs',
  databases: 'Databases',
  dependencies: 'Dependencies'
};

function docLabel(cat: string): string {
  return DOC_LABELS[cat] ?? cat.charAt(0).toUpperCase() + cat.slice(1);
}

interface SidebarProps {
  nav: NavData | null;
  onOpenSearch: () => void;
}

export function Sidebar({ nav, onOpenSearch }: SidebarProps): preact.JSX.Element {
  const currentPath = useCurrentPath();

  return (
    <nav class="w-64 min-w-[16rem] bg-latte-mantle dark:bg-mocha-mantle border-r border-latte-surface0 dark:border-mocha-surface0 p-4 sticky top-0 h-screen overflow-y-auto shrink-0">
      <a
        href="/"
        onClick={(e: Event) => {
          e.preventDefault();
          route('/');
        }}
        class="text-xl font-bold text-latte-blue dark:text-mocha-blue mb-6 block"
      >
        nosce
      </a>

      <ProfileSelector />

      {/* Command palette trigger */}
      <button
        onClick={onOpenSearch}
        class="w-full flex items-center gap-2 px-3 py-2 mb-2 text-sm text-latte-overlay0 dark:text-mocha-overlay0 bg-latte-surface0/50 dark:bg-mocha-surface0/50 border border-latte-surface0 dark:border-mocha-surface0 rounded-md hover:border-latte-blue/50 dark:hover:border-mocha-blue/50 transition-colors cursor-pointer"
      >
        <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path
            stroke-linecap="round"
            stroke-linejoin="round"
            stroke-width="2"
            d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
          />
        </svg>
        <span class="flex-1 text-left">Search...</span>
        <kbd class="hidden sm:inline text-xs px-1.5 py-0.5 bg-latte-surface0 dark:bg-mocha-surface0 rounded">
          {'\u2318'}K
        </kbd>
      </button>

      <SectionTitle>Reports</SectionTitle>
      <NavLink href="/reports" active={currentPath === '/reports'}>
        All reports
      </NavLink>

      <SectionTitle>Documentation</SectionTitle>
      {nav?.docs && nav.docs.length > 0 ? (
        nav.docs.map((cat) => (
          <NavLink key={cat} href={`/docs/${cat}`} active={currentPath === `/docs/${cat}`}>
            {docLabel(cat)}
          </NavLink>
        ))
      ) : (
        <>
          <NavLink href="/docs/overview" active={currentPath === '/docs/overview'}>
            Overview
          </NavLink>
          <NavLink href="/docs/architecture" active={currentPath === '/docs/architecture'}>
            Architecture
          </NavLink>
          <NavLink href="/docs/apis" active={currentPath === '/docs/apis'}>
            APIs
          </NavLink>
          <NavLink href="/docs/databases" active={currentPath === '/docs/databases'}>
            Databases
          </NavLink>
          <NavLink href="/docs/dependencies" active={currentPath === '/docs/dependencies'}>
            Dependencies
          </NavLink>
        </>
      )}

      {nav?.submodules && nav.submodules.length > 0 && (
        <>
          <SectionTitle>Submodules</SectionTitle>
          {nav.submodules.map((sub) => (
            <SubmoduleTree key={sub.name} sub={sub} currentPath={currentPath} />
          ))}
        </>
      )}
    </nav>
  );
}
