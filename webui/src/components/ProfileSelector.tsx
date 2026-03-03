import { useProfile } from '../context/ProfileContext';

export function ProfileSelector(): preact.JSX.Element | null {
  const { profileId, setProfileId, profiles } = useProfile();

  if (profiles.length === 0) {
    return null;
  }

  return (
    <div class="mb-4">
      <label class="text-xs font-semibold uppercase tracking-wider text-latte-overlay0 dark:text-mocha-overlay0 block mb-1 px-1">
        Profile
      </label>
      <select
        value={profileId}
        onChange={(e: Event) => {
          const target = e.target as HTMLSelectElement;
          setProfileId(target.value);
        }}
        class="w-full px-3 py-2 bg-latte-surface0 dark:bg-mocha-surface0 border border-latte-surface1 dark:border-mocha-surface1 rounded-md text-sm text-latte-text dark:text-mocha-text focus:border-latte-blue dark:focus:border-mocha-blue focus:outline-none cursor-pointer"
      >
        {profiles.map((p) => (
          <option key={p.id} value={p.id}>
            {p.label}
          </option>
        ))}
      </select>
    </div>
  );
}
