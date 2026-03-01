import type { ComponentChildren } from 'preact';
import { createContext } from 'preact';
import { useContext, useState } from 'preact/hooks';

import type { ProfileInfo } from '../types';

const STORAGE_KEY = 'nosce-profile';
const DEFAULT_PROFILE = 'engineer';

interface ProfileContextValue {
  profileId: string;
  setProfileId: (id: string) => void;
  profiles: ProfileInfo[];
  currentProfile: ProfileInfo | undefined;
}

// eslint-disable-next-line @typescript-eslint/no-empty-function
const noop = (): void => {};

const ProfileContext = createContext<ProfileContextValue>({
  profileId: DEFAULT_PROFILE,
  setProfileId: noop,
  profiles: [],
  currentProfile: undefined,
});

export function useProfile(): ProfileContextValue {
  return useContext(ProfileContext);
}

interface ProfileProviderProps {
  profiles: ProfileInfo[];
  children: ComponentChildren;
}

export function ProfileProvider({
  profiles,
  children,
}: ProfileProviderProps): preact.JSX.Element {
  const [profileId, setProfileIdRaw] = useState<string>(() => {
    try {
      return localStorage.getItem(STORAGE_KEY) ?? DEFAULT_PROFILE;
    } catch {
      return DEFAULT_PROFILE;
    }
  });

  const setProfileId = (id: string): void => {
    try {
      localStorage.setItem(STORAGE_KEY, id);
    } catch {
      // localStorage unavailable
    }
    setProfileIdRaw(id);
  };

  const currentProfile = profiles.find((p) => p.id === profileId);

  return (
    <ProfileContext.Provider
      value={{ profileId, setProfileId, profiles, currentProfile }}
    >
      {children}
    </ProfileContext.Provider>
  );
}
