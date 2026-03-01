import { useCallback, useEffect, useState } from 'preact/hooks';

import { apiUrl } from '../api';
import type { MediaItem, MediaManifest } from '../types';

interface MediaGalleryProps {
  date: string;
}

interface GroupedMedia {
  repo: string;
  prs: {
    pr_number: number;
    pr_title: string;
    author: string;
    items: MediaItem[];
  }[];
}

function groupByRepoPr(items: MediaItem[]): GroupedMedia[] {
  const repoMap = new Map<string, Map<number, { pr_title: string; author: string; items: MediaItem[] }>>();

  for (const item of items) {
    if (!repoMap.has(item.repo)) {
      repoMap.set(item.repo, new Map());
    }
    const prMap = repoMap.get(item.repo)!;
    if (!prMap.has(item.pr_number)) {
      prMap.set(item.pr_number, { pr_title: item.pr_title, author: item.author, items: [] });
    }
    prMap.get(item.pr_number)!.items.push(item);
  }

  const groups: GroupedMedia[] = [];
  for (const [repo, prMap] of repoMap) {
    const prs = Array.from(prMap.entries()).map(([pr_number, data]) => ({
      pr_number,
      ...data,
    }));
    groups.push({ repo, prs });
  }
  return groups;
}

function Lightbox({
  images,
  index,
  date,
  onClose,
  onNav,
}: {
  images: MediaItem[];
  index: number;
  date: string;
  onClose: () => void;
  onNav: (idx: number) => void;
}) {
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
      if (e.key === 'ArrowLeft' && index > 0) onNav(index - 1);
      if (e.key === 'ArrowRight' && index < images.length - 1) onNav(index + 1);
    };
    document.addEventListener('keydown', handler);
    return () => document.removeEventListener('keydown', handler);
  }, [index, images.length, onClose, onNav]);

  const item = images[index];
  if (!item) return null;

  return (
    <div class="media-lightbox" onClick={onClose}>
      <button class="media-lightbox-close" onClick={onClose} aria-label="Close">
        &times;
      </button>
      {index > 0 && (
        <button
          class="media-lightbox-nav media-lightbox-prev"
          onClick={(e) => { e.stopPropagation(); onNav(index - 1); }}
          aria-label="Previous"
        >
          &#8249;
        </button>
      )}
      {index < images.length - 1 && (
        <button
          class="media-lightbox-nav media-lightbox-next"
          onClick={(e) => { e.stopPropagation(); onNav(index + 1); }}
          aria-label="Next"
        >
          &#8250;
        </button>
      )}
      <img
        src={apiUrl(`/api/media/${date}/${item.filename}`)}
        alt={item.alt || item.pr_title}
        onClick={(e) => e.stopPropagation()}
      />
      <div class="media-lightbox-caption">
        <strong>#{item.pr_number}</strong> &mdash; {item.pr_title}
        {item.alt && <span> &middot; {item.alt}</span>}
      </div>
    </div>
  );
}

export function MediaGallery({ date }: MediaGalleryProps): preact.JSX.Element | null {
  const [manifest, setManifest] = useState<MediaManifest | null>(null);
  const [expanded, setExpanded] = useState(false);
  const [lightboxIndex, setLightboxIndex] = useState<number | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    setLoading(true);
    setManifest(null);
    setExpanded(false);
    setLightboxIndex(null);

    fetch(apiUrl(`/api/media/${date}`))
      .then((res) => res.json())
      .then((data: MediaManifest) => {
        setManifest(data);
        setLoading(false);
      })
      .catch(() => {
        setManifest(null);
        setLoading(false);
      });
  }, [date]);

  const allImages = useCallback(() => {
    if (!manifest) return [];
    return manifest.items.filter((i) => i.type === 'image');
  }, [manifest]);

  const openLightbox = useCallback(
    (item: MediaItem) => {
      const images = allImages();
      const idx = images.findIndex((i) => i.filename === item.filename);
      if (idx !== -1) setLightboxIndex(idx);
    },
    [allImages],
  );

  if (loading || !manifest || manifest.items.length === 0) return null;

  const imageCount = manifest.items.filter((i) => i.type === 'image').length;
  const videoCount = manifest.items.filter((i) => i.type === 'video').length;
  const parts: string[] = [];
  if (imageCount > 0) parts.push(`${imageCount} image${imageCount !== 1 ? 's' : ''}`);
  if (videoCount > 0) parts.push(`${videoCount} video${videoCount !== 1 ? 's' : ''}`);

  const groups = groupByRepoPr(manifest.items);

  return (
    <>
      <div class="media-gallery">
        <div class="media-gallery-header" onClick={() => setExpanded(!expanded)}>
          <div class="flex items-center gap-2">
            <span
              class="text-xs transition-transform"
              style={{ transform: expanded ? 'rotate(90deg)' : 'rotate(0)' }}
            >
              &#9654;
            </span>
            <span class="text-sm font-medium text-latte-text dark:text-mocha-text">
              Screenshots & Videos
            </span>
            <span class="media-gallery-badge">{parts.join(', ')}</span>
          </div>
        </div>
        {expanded && (
          <div>
            {groups.map((group) => (
              <div key={group.repo}>
                {groups.length > 1 && (
                  <div class="text-xs font-semibold text-latte-overlay1 dark:text-mocha-overlay1 uppercase tracking-wider px-4 pt-2 pb-1">
                    {group.repo}
                  </div>
                )}
                {group.prs.map((pr) => (
                  <div key={`${group.repo}-${pr.pr_number}`} class="media-pr-group">
                    <div class="media-pr-label">
                      <strong>#{pr.pr_number}</strong> &mdash; {pr.pr_title}
                    </div>
                    <div class="media-gallery-grid">
                      {pr.items.map((item) =>
                        item.type === 'image' ? (
                          <div
                            key={item.filename}
                            class="media-thumbnail-wrapper"
                            onClick={() => openLightbox(item)}
                          >
                            <img
                              class="media-thumbnail"
                              src={apiUrl(`/api/media/${date}/${item.filename}`)}
                              alt={item.alt || item.pr_title}
                              loading="lazy"
                            />
                            <div class="media-thumbnail-caption">
                              {item.alt || item.filename}
                            </div>
                          </div>
                        ) : (
                          <div key={item.filename} class="media-thumbnail-wrapper">
                            <video
                              class="media-video"
                              src={apiUrl(`/api/media/${date}/${item.filename}`)}
                              controls
                              preload="metadata"
                            />
                            <div class="media-thumbnail-caption">
                              {item.alt || item.filename}
                            </div>
                          </div>
                        ),
                      )}
                    </div>
                  </div>
                ))}
              </div>
            ))}
          </div>
        )}
      </div>

      {lightboxIndex !== null && (
        <Lightbox
          images={allImages()}
          index={lightboxIndex}
          date={date}
          onClose={() => setLightboxIndex(null)}
          onNav={setLightboxIndex}
        />
      )}
    </>
  );
}
