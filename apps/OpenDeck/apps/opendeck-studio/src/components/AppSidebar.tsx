import { memo } from 'react';
import { Glyph, type GlyphName } from './Glyph';

export type AppSection = 'buttons'|'dials'|'touch'|'profiles'|'plugins'|'settings';

interface AppSidebarProps {
  section: AppSection;
  onSectionChange: (section: AppSection) => void;
}

const ITEMS: Array<{ id: AppSection; label: string; sublabel: string; icon: GlyphName }> = [
  { id: 'buttons', label: 'Buttons', sublabel: 'Configure keys', icon: 'grid' },
  { id: 'dials', label: 'Dials', sublabel: 'Configure dials', icon: 'dial' },
  { id: 'touch', label: 'Touch Strip', sublabel: 'Configure touch strip', icon: 'touch' },
  { id: 'profiles', label: 'Profiles', sublabel: 'Manage profiles', icon: 'profiles' },
  { id: 'plugins', label: 'Plugins', sublabel: 'Browse & install', icon: 'plugins' },
  { id: 'settings', label: 'Settings', sublabel: 'App preferences', icon: 'settings' },
];

export const AppSidebar = memo(function AppSidebar({ section, onSectionChange }: AppSidebarProps) {
  return <aside className="app-sidebar" data-layout-region="sidebar">
    <nav className="sidebar-nav" aria-label="OpenDeck sections">
      {ITEMS.map((item) => <button
        key={item.id}
        className={`sidebar-item${section === item.id ? ' active' : ''}`}
        aria-current={section === item.id ? 'page' : undefined}
        aria-label={item.label}
        onClick={() => onSectionChange(item.id)}
      >
        <span className="sidebar-icon"><Glyph name={item.icon} /></span>
        <span className="sidebar-copy"><strong>{item.label}</strong><small>{item.sublabel}</small></span>
      </button>)}
    </nav>
    <footer className="sidebar-footer"><span className="sidebar-heart">♥</span><div><strong>OpenDeck+ 2.0.33</strong><small>Open Source. More possibilities.</small></div></footer>
  </aside>;
});
