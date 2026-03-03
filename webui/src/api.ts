/** Base path prefix injected by the server when BASE_PATH is set. */
const BASE: string =
  (window as unknown as Record<string, string>)['__NOSCE_BASE__'] ?? '';

/** Prepend the base path to an API URL (e.g. "/api/nav" → "/nosce/api/nav"). */
export function apiUrl(path: string): string {
  return `${BASE}${path}`;
}
