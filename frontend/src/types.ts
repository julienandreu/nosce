/** Profile definition returned by the backend */
export interface ProfileInfo {
  id: string;
  label: string;
  icon: string;
  description: string;
}

/** TOC entry extracted from markdown headings */
export interface TocEntry {
  id: string;
  text: string;
  level: number;
}

/** Submodule with its packages */
export interface SubmoduleNav {
  name: string;
  packages: string[];
}

/** Navigation data returned by /api/nav */
export interface NavData {
  latest_report: string | null;
  reports: string[];
  docs: string[];
  submodules: SubmoduleNav[];
  profiles: ProfileInfo[];
  media_dates: string[];
}

/** Single media item from a PR */
export interface MediaItem {
  filename: string;
  type: 'image' | 'video';
  repo: string;
  pr_number: number;
  pr_title: string;
  author: string;
  alt: string;
  original_url: string;
}

/** Media manifest for a given date, returned by /api/media/:date */
export interface MediaManifest {
  date: string;
  items: MediaItem[];
}

/** Markdown content returned by /api/reports/:date, /api/docs/:category, /api/submodules/:name */
export interface MarkdownData {
  html: string;
  raw: string;
  /** Which profile was served. null means the base (full) report. */
  profile: string | null;
  toc: TocEntry[];
}

/** Single report entry returned by /api/reports */
export interface ReportEntry {
  id: string;
  label: string;
  date_range: string;
  tldr: string;
  tags: string[];
  commits: number;
  repos: string[];
}

/** Reports list returned by /api/reports */
export interface ReportsData {
  reports: ReportEntry[];
}

/** Single search result */
export interface SearchHit {
  file: string;
  url: string;
  line: number;
  context: string;
  heading: string | null;
}

/** Search results returned by /api/search */
export interface SearchData {
  results: SearchHit[];
}
