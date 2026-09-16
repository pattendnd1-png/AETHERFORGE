import type { ReactElement, SVGProps } from 'react';

export type GlyphName =
  | 'grid'|'dial'|'touch'|'profiles'|'plugins'|'settings'|'device'|'profile'
  | 'undo'|'redo'|'link'|'market'|'search'|'chevron'|'scene'|'mic'|'music'
  | 'browser'|'obs'|'chat'|'discord'|'system'|'volume'|'zoom'|'brightness'|'media'
  | 'folder'|'next'|'previous'|'switch'|'blank'|'audio'|'video'|'record';

interface Props extends SVGProps<SVGSVGElement> { name: GlyphName }

export function Glyph({ name, ...props }: Props) {
  const common = { fill: 'none', stroke: 'currentColor', strokeWidth: 1.8, strokeLinecap: 'round' as const, strokeLinejoin: 'round' as const };
  const paths: Record<GlyphName, ReactElement> = {
    grid: <><rect x="3" y="3" width="7" height="7" rx="1.5"/><rect x="14" y="3" width="7" height="7" rx="1.5"/><rect x="3" y="14" width="7" height="7" rx="1.5"/><rect x="14" y="14" width="7" height="7" rx="1.5"/></>,
    dial: <><circle cx="12" cy="12" r="8"/><path d="M12 4v3M18 6l-2 2M20 12h-3"/><circle cx="12" cy="12" r="2.5"/></>,
    touch: <><rect x="3" y="7" width="18" height="10" rx="5"/><circle cx="9" cy="12" r="1.5"/><path d="M13 12h5"/></>,
    profiles: <><path d="M3 8h7l2 2h9v9H3z"/><path d="M3 8V5h7l2 3"/></>,
    plugins: <><path d="M8 3v5H3v5h5v5h5v-5h5V8h-5V3z"/><path d="M8 8h5v5H8z"/></>,
    settings: <><circle cx="12" cy="12" r="3"/><path d="M12 2v3M12 19v3M2 12h3M19 12h3M4.9 4.9L7 7M17 17l2.1 2.1M19.1 4.9L17 7M7 17l-2.1 2.1"/></>,
    device: <><rect x="3" y="5" width="18" height="14" rx="3"/><path d="M7 9h4v4H7zM14 9h3M14 13h3"/></>,
    profile: <><circle cx="12" cy="8" r="3"/><path d="M5 20c1-4 3-6 7-6s6 2 7 6"/></>,
    undo: <><path d="M9 7H4v-5"/><path d="M4 7c3-4 9-5 13-1s3 10-1 13"/></>,
    redo: <><path d="M15 7h5v-5"/><path d="M20 7c-3-4-9-5-13-1s-3 10 1 13"/></>,
    link: <><path d="M10 13l4-4"/><path d="M8 16H6a4 4 0 010-8h4M16 8h2a4 4 0 010 8h-4"/></>,
    market: <><path d="M4 8h16l-1 12H5z"/><path d="M7 8V5h10v3M8 12h8"/></>,
    search: <><circle cx="10" cy="10" r="6"/><path d="M15 15l6 6"/></>,
    chevron: <path d="M8 10l4 4 4-4"/>,
    scene: <><circle cx="12" cy="12" r="4"/><path d="M12 2v4M12 18v4M2 12h4M18 12h4M5 5l3 3M16 16l3 3M19 5l-3 3M8 16l-3 3"/></>,
    mic: <><rect x="9" y="3" width="6" height="11" rx="3"/><path d="M6 11a6 6 0 0012 0M12 17v4M9 21h6"/></>,
    music: <><circle cx="9" cy="17" r="3"/><circle cx="17" cy="15" r="3"/><path d="M12 17V5l8-2v12"/></>,
    browser: <><circle cx="12" cy="12" r="9"/><path d="M3 12h18M12 3c3 3 4 6 4 9s-1 6-4 9M12 3c-3 3-4 6-4 9s1 6 4 9"/></>,
    obs: <><circle cx="12" cy="12" r="9"/><path d="M12 5c3 0 5 2 5 5-2-1-4-1-6 1M6 15c-2-3-1-6 2-7 0 2 1 4 3 5M16 16c-1 3-5 4-7 2 2-1 3-3 3-5"/></>,
    chat: <><path d="M4 5h16v11H9l-5 4z"/><path d="M8 9h8M8 12h6"/></>,
    discord: <><path d="M6 7c4-3 8-3 12 0l2 10c-3 3-5 3-7 1M11 18c-2 2-4 2-7-1L6 7"/><circle cx="9" cy="12" r="1"/><circle cx="15" cy="12" r="1"/></>,
    system: <><circle cx="12" cy="12" r="3"/><path d="M12 3v3M12 18v3M3 12h3M18 12h3M5.6 5.6l2.1 2.1M16.3 16.3l2.1 2.1M18.4 5.6l-2.1 2.1M7.7 16.3l-2.1 2.1"/></>,
    volume: <><path d="M4 10h4l5-4v12l-5-4H4z"/><path d="M16 9c2 2 2 4 0 6M19 6c4 4 4 8 0 12"/></>,
    zoom: <><rect x="3" y="6" width="13" height="12" rx="2"/><path d="M16 10l5-3v10l-5-3z"/></>,
    brightness: <><circle cx="12" cy="12" r="4"/><path d="M12 2v3M12 19v3M2 12h3M19 12h3M5 5l2 2M17 17l2 2M19 5l-2 2M7 17l-2 2"/></>,
    media: <><path d="M7 5l10 7-10 7z"/><path d="M19 5v14"/></>,
    folder: <path d="M3 7h7l2 2h9v10H3z"/>,
    next: <><path d="M8 5l7 7-7 7"/><path d="M15 5v14"/></>,
    previous: <><path d="M16 5l-7 7 7 7"/><path d="M9 5v14"/></>,
    switch: <><path d="M4 8h13l-3-3M20 16H7l3 3"/></>,
    blank: <rect x="5" y="5" width="14" height="14" rx="2"/>,
    audio: <><path d="M4 10h4l5-4v12l-5-4H4z"/><path d="M17 9l3 6"/></>,
    video: <><rect x="3" y="6" width="13" height="12" rx="2"/><path d="M16 10l5-3v10l-5-3z"/></>,
    record: <circle cx="12" cy="12" r="7"/>,
  };
  return <svg viewBox="0 0 24 24" aria-hidden="true" {...common} {...props}>{paths[name]}</svg>;
}

export function glyphForLabel(label: string): GlyphName {
  const value = label.toLowerCase();
  if (value.includes('scene')) return 'scene';
  if (value.includes('mic') || value.includes('mute')) return 'mic';
  if (value.includes('spotify') || value.includes('music')) return 'music';
  if (value.includes('browser')) return 'browser';
  if (value === 'obs' || value.includes('obs')) return 'obs';
  if (value.includes('twitch')) return 'chat';
  if (value.includes('discord')) return 'discord';
  if (value.includes('system')) return 'system';
  if (value.includes('volume') || value.includes('audio')) return 'volume';
  if (value.includes('zoom')) return 'zoom';
  if (value.includes('bright')) return 'brightness';
  if (value.includes('media')) return 'media';
  if (value.includes('folder')) return 'folder';
  if (value.includes('next')) return 'next';
  if (value.includes('previous')) return 'previous';
  if (value.includes('switch')) return 'switch';
  if (value.includes('market')) return 'market';
  if (value.includes('record')) return 'record';
  return 'blank';
}
