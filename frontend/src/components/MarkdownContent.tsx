import { useEffect, useRef, useState } from 'preact/hooks';
import mermaid from 'mermaid';

import type { MarkdownData, TocEntry } from '../types';

mermaid.initialize({
  startOnLoad: false,
  theme: window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'default',
  securityLevel: 'loose',
});

// Update mermaid theme when OS preference changes
window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => {
  mermaid.initialize({
    startOnLoad: false,
    theme: e.matches ? 'dark' : 'default',
    securityLevel: 'loose',
  });
});

function attachPanZoom(container: HTMLElement): void {
  const wrappers = container.querySelectorAll<HTMLElement>('.mermaid-panzoom');
  wrappers.forEach((wrapper) => {
    const inner = wrapper.querySelector<HTMLElement>('.mermaid-panzoom-inner');
    if (!inner) return;

    let scale = 1;
    let panX = 0;
    let panY = 0;
    let isPanning = false;
    let startX = 0;
    let startY = 0;

    const applyTransform = (): void => {
      inner.style.transform = `translate(${String(panX)}px, ${String(panY)}px) scale(${String(scale)})`;
    };

    wrapper.addEventListener('wheel', (e) => {
      e.preventDefault();
      const delta = e.deltaY > 0 ? 0.9 : 1.1;
      const newScale = Math.min(Math.max(scale * delta, 0.2), 5);

      // Zoom toward cursor position
      const rect = wrapper.getBoundingClientRect();
      const cx = e.clientX - rect.left;
      const cy = e.clientY - rect.top;
      panX = cx - ((cx - panX) * newScale) / scale;
      panY = cy - ((cy - panY) * newScale) / scale;
      scale = newScale;
      applyTransform();
    });

    wrapper.addEventListener('mousedown', (e) => {
      if (e.button !== 0) return;
      isPanning = true;
      startX = e.clientX - panX;
      startY = e.clientY - panY;
      wrapper.style.cursor = 'grabbing';
    });

    window.addEventListener('mousemove', (e) => {
      if (!isPanning) return;
      panX = e.clientX - startX;
      panY = e.clientY - startY;
      applyTransform();
    });

    window.addEventListener('mouseup', () => {
      if (!isPanning) return;
      isPanning = false;
      wrapper.style.cursor = 'grab';
    });

    // Double-click to reset
    wrapper.addEventListener('dblclick', () => {
      scale = 1;
      panX = 0;
      panY = 0;
      applyTransform();
    });
  });
}

function attachCopyButtons(container: HTMLElement): void {
  const codeBlocks = container.querySelectorAll('pre');
  codeBlocks.forEach((pre) => {
    // Skip mermaid blocks
    const code = pre.querySelector('code');
    if (!code) return;
    if (code.classList.contains('language-mermaid')) return;
    // Skip if already has copy button
    if (pre.querySelector('.copy-btn')) return;

    pre.style.position = 'relative';

    const btn = document.createElement('button');
    btn.className = 'copy-btn';
    btn.textContent = 'Copy';
    btn.addEventListener('click', () => {
      const text = code.textContent ?? '';
      void navigator.clipboard.writeText(text).then(() => {
        btn.textContent = 'Copied!';
        btn.classList.add('copied');
        setTimeout(() => {
          btn.textContent = 'Copy';
          btn.classList.remove('copied');
        }, 2000);
      });
    });

    pre.appendChild(btn);
  });
}

interface MarkdownContentProps {
  url: string;
  onToc?: (toc: TocEntry[]) => void;
  onProfile?: (profile: string | null) => void;
}

export function MarkdownContent({ url, onToc, onProfile }: MarkdownContentProps): preact.JSX.Element {
  const [html, setHtml] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    setLoading(true);
    setError(null);

    fetch(url)
      .then((r) => {
        if (!r.ok) {
          throw new Error(`Not found (${String(r.status)})`);
        }

        return r.json() as Promise<MarkdownData>;
      })
      .then((data) => {
        setHtml(data.html);
        setLoading(false);
        if (onToc) {
          onToc(data.toc);
        }
        if (onProfile) {
          onProfile(data.profile);
        }
      })
      .catch((e: unknown) => {
        setError(e instanceof Error ? e.message : 'Unknown error');
        setLoading(false);
        if (onToc) {
          onToc([]);
        }
        if (onProfile) {
          onProfile(null);
        }
      });
  }, [url, onToc, onProfile]);

  useEffect(() => {
    if (!html || !containerRef.current) return;

    const codeBlocks = containerRef.current.querySelectorAll('code.language-mermaid');
    codeBlocks.forEach((code) => {
      const pre = code.parentElement;
      if (!pre || pre.tagName !== 'PRE') return;

      const mermaidDiv = document.createElement('div');
      mermaidDiv.className = 'mermaid';
      mermaidDiv.textContent = code.textContent ?? '';
      pre.replaceWith(mermaidDiv);
    });

    void mermaid
      .run({ nodes: containerRef.current.querySelectorAll('.mermaid') })
      .then(() => {
        if (!containerRef.current) return;
        // Wrap each rendered mermaid diagram with pan-zoom container
        const diagrams = containerRef.current.querySelectorAll<HTMLElement>('.mermaid');
        diagrams.forEach((diagram) => {
          if (diagram.closest('.mermaid-panzoom')) return;

          const wrapper = document.createElement('div');
          wrapper.className = 'mermaid-panzoom';

          const inner = document.createElement('div');
          inner.className = 'mermaid-panzoom-inner';

          // Move SVG content into the inner container
          while (diagram.firstChild) {
            inner.appendChild(diagram.firstChild);
          }

          wrapper.appendChild(inner);
          diagram.appendChild(wrapper);
        });

        attachPanZoom(containerRef.current);
      });

    // Attach copy buttons to code blocks (non-mermaid)
    attachCopyButtons(containerRef.current);
  }, [html]);

  if (loading) {
    return (
      <div class="animate-pulse space-y-3">
        <div class="h-8 bg-latte-surface0 dark:bg-mocha-surface0 rounded w-2/3" />
        <div class="h-4 bg-latte-surface0 dark:bg-mocha-surface0 rounded w-full" />
        <div class="h-4 bg-latte-surface0 dark:bg-mocha-surface0 rounded w-5/6" />
        <div class="h-4 bg-latte-surface0 dark:bg-mocha-surface0 rounded w-4/5" />
      </div>
    );
  }

  if (error) {
    return (
      <div class="text-center mt-16 text-latte-subtext0 dark:text-mocha-subtext0">
        <p class="text-lg">{error}</p>
        <p class="mt-2 text-sm">
          Run <code class="bg-latte-surface0 dark:bg-mocha-surface0 px-2 py-0.5 rounded">/sync</code> or{' '}
          <code class="bg-latte-surface0 dark:bg-mocha-surface0 px-2 py-0.5 rounded">/docs</code> to generate content.
        </p>
      </div>
    );
  }

  if (!html) {
    return <div />;
  }

  return <div ref={containerRef} class="markdown-body" dangerouslySetInnerHTML={{ __html: html }} />;
}
