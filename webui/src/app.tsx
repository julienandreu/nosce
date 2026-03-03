import Router from 'preact-router';
import { useCallback, useEffect, useState } from 'preact/hooks';

import { apiUrl } from './api';
import { CommandPalette } from './components/CommandPalette';
import { Sidebar } from './components/Sidebar';
import { ProfileProvider } from './context/ProfileContext';
import { DocView } from './pages/DocView';
import { Home } from './pages/Home';
import { PackageView } from './pages/PackageView';
import { ReportView } from './pages/ReportView';
import { ReportsList } from './pages/ReportsList';
import { Search } from './pages/Search';
import { SubmoduleView } from './pages/SubmoduleView';
import type { NavData } from './types';

export function App(): preact.JSX.Element {
  const [nav, setNav] = useState<NavData | null>(null);
  const [searchOpen, setSearchOpen] = useState(false);

  useEffect(() => {
    fetch(apiUrl('/api/nav'))
      .then((r) => r.json() as Promise<NavData>)
      .then(setNav)
      .catch(() => {
        setNav({
          latest_report: null,
          reports: [],
          docs: [],
          submodules: [],
          profiles: [],
          media_dates: []
        });
      });
  }, []);

  // Cmd+K / Ctrl+K shortcut
  useEffect(() => {
    const handler = (e: KeyboardEvent): void => {
      if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
        e.preventDefault();
        setSearchOpen((open) => !open);
      }
    };
    window.addEventListener('keydown', handler);
    return () => {
      window.removeEventListener('keydown', handler);
    };
  }, []);

  const openSearch = useCallback(() => {
    setSearchOpen(true);
  }, []);
  const closeSearch = useCallback(() => {
    setSearchOpen(false);
  }, []);

  return (
    <ProfileProvider profiles={nav?.profiles ?? []}>
      <div class="flex min-h-screen">
        <Sidebar nav={nav} onOpenSearch={openSearch} />
        <main class="flex-1 flex p-8">
          <Router>
            <Home path="/" />
            <ReportsList path="/reports" />
            <ReportView path="/reports/:date" />
            <DocView path="/docs/:category" />
            <PackageView path="/submodules/:name/packages/:pkg" />
            <SubmoduleView path="/submodules/:name" />
            <Search path="/search" />
          </Router>
        </main>
      </div>
      <CommandPalette open={searchOpen} onClose={closeSearch} />
    </ProfileProvider>
  );
}
