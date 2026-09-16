import { memo, useDeferredValue, useMemo, useState } from 'react';
import { ACTION_DEFINITIONS, supportsControl, type ActionDefinition } from '../model/actions';
import { Glyph, glyphForLabel, type GlyphName } from './Glyph';
import { VirtualActionList } from './VirtualActionList';

const ACTION_MIME = 'application/x-opendeck-action';
export type ActionLibraryMode = 'keys' | 'dials';
interface ActionLibraryProps { mode: ActionLibraryMode; collapsed: boolean; onModeChange: (mode: ActionLibraryMode) => void; onChoose: (id: string) => void; onToggleCollapsed: () => void; qualificationCatalogSize?: number; }
type Row = { kind: 'group'; group: string; collapsed: boolean } | { kind: 'action'; action: ActionDefinition };
const EXTRA_GROUPS = ['System', 'Audio', 'Media', 'Productivity', 'Integrations'];
function groupGlyph(group: string): GlyphName { const v=group.toLowerCase(); if(v.includes('obs')) return 'obs'; if(v.includes('elgato')) return 'market'; if(v.includes('nav')) return 'switch'; if(v.includes('audio')) return 'audio'; if(v.includes('media')) return 'media'; if(v.includes('system')) return 'system'; if(v.includes('integr')) return 'link'; return 'grid'; }

export const ActionLibrary = memo(function ActionLibrary({ mode, collapsed, onModeChange, onChoose, onToggleCollapsed, qualificationCatalogSize = 0 }: ActionLibraryProps) {
  const [query, setQuery] = useState('');
  const deferredQuery = useDeferredValue(query);
  const [collapsedGroups, setCollapsedGroups] = useState<Set<string>>(() => new Set(EXTRA_GROUPS));
  const rows = useMemo<Row[]>(() => {
    const needle = deferredQuery.trim().toLowerCase();
    const catalog: ActionDefinition[] = qualificationCatalogSize > ACTION_DEFINITIONS.length
      ? Array.from({ length: qualificationCatalogSize }, (_, index) => {
          const base = ACTION_DEFINITIONS[index % ACTION_DEFINITIONS.length];
          return { ...base, label: `${base.label} Synthetic ${index}` };
        })
      : ACTION_DEFINITIONS;
    const filtered = catalog.filter((item) => `${item.group} ${item.label}`.toLowerCase().includes(needle) && (mode === 'keys' ? supportsControl(item.id, 'key') : supportsControl(item.id, 'dial')));
    const groups = new Map<string, ActionDefinition[]>();
    for (const item of filtered) groups.set(item.group, [...(groups.get(item.group) ?? []), item]);
    for (const group of EXTRA_GROUPS) if (!needle && !groups.has(group)) groups.set(group, []);
    const result: Row[] = [];
    for (const [group, items] of groups) {
      const isCollapsed = collapsedGroups.has(group);
      result.push({ kind: 'group', group, collapsed: isCollapsed });
      if (!isCollapsed) for (const action of items) result.push({ kind: 'action', action });
    }
    return result;
  }, [mode, deferredQuery, collapsedGroups, qualificationCatalogSize]);
  function toggleGroup(group: string) { setCollapsedGroups((current) => { const next=new Set(current); if(next.has(group)) next.delete(group); else next.add(group); return next; }); }
  if (collapsed) return <aside className="action-library collapsed" data-layout-region="actions" aria-label="Actions"><button className="panel-expand" aria-label="Expand action panel" title="Expand Actions" onClick={onToggleCollapsed}>‹</button></aside>;
  return <aside className="action-library" data-layout-region="actions" aria-label="Actions">
    <div className="action-library-title"><div><strong>Action Library</strong><span>Drag actions onto controls</span></div><button className="panel-collapse" aria-label="Collapse action panel" title="Collapse Actions" onClick={onToggleCollapsed}>›</button></div>
    <label className="action-search"><Glyph name="search"/><input aria-label="Search actions" placeholder="Search actions…" value={query} onChange={(event) => setQuery(event.target.value)} /><kbd>⌘ K</kbd></label>
    <div className="action-mode-tabs" role="tablist" aria-label="Action modes"><button role="tab" aria-selected={mode === 'keys'} className={mode === 'keys' ? 'active' : ''} onClick={() => onModeChange('keys')}>Keys</button><button role="tab" aria-selected={mode === 'dials'} className={mode === 'dials' ? 'active' : ''} onClick={() => onModeChange('dials')}>Dials</button></div>
    <div className="action-scroll"><VirtualActionList items={rows} rowHeight={42} viewportHeight={690} renderRow={(row) => row.kind === 'group' ? <button className="action-group-heading" aria-expanded={!row.collapsed} onClick={() => toggleGroup(row.group)}><span className="group-icon"><Glyph name={groupGlyph(row.group)} /></span><strong>{row.group}</strong><span className="group-chevron">{row.collapsed ? '⌄' : '⌃'}</span></button> : <button key={row.action.id} draggable className="action-entry" title={row.action.label} onDragStart={(event) => { event.dataTransfer.effectAllowed='copy'; event.dataTransfer.setData(ACTION_MIME,row.action.id); }} onClick={() => onChoose(row.action.id)}><span className="action-entry-icon"><Glyph name={glyphForLabel(row.action.label)} /></span><span>{row.action.label}</span></button>} /></div>
  </aside>;
});
