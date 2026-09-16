import { useMemo, useState } from 'react';
import { ACTION_DEFINITIONS, supportsControl } from '../model/actions';

const ACTION_MIME = 'application/x-opendeck-action';

export type ActionLibraryMode = 'keys' | 'dials';

interface ActionLibraryProps {
  mode: ActionLibraryMode;
  collapsed: boolean;
  onModeChange: (mode: ActionLibraryMode) => void;
  onChoose: (id: string) => void;
  onToggleCollapsed: () => void;
}

export function ActionLibrary({ mode, collapsed, onModeChange, onChoose, onToggleCollapsed }: ActionLibraryProps) {
  const [query, setQuery] = useState('');
  const [collapsedGroups, setCollapsedGroups] = useState<Set<string>>(() => new Set());
  const groups = useMemo(() => {
    const needle = query.trim().toLowerCase();
    const filtered = ACTION_DEFINITIONS.filter((item) => {
      const matchesSearch = `${item.group} ${item.label}`.toLowerCase().includes(needle);
      const matchesMode = mode === 'keys'
        ? supportsControl(item.id, 'key')
        : supportsControl(item.id, 'dial');
      return matchesSearch && matchesMode;
    });
    const grouped = new Map<string, typeof ACTION_DEFINITIONS>();
    for (const item of filtered) {
      const existing = grouped.get(item.group) ?? [];
      grouped.set(item.group, [...existing, item]);
    }
    return grouped;
  }, [mode, query]);

  function toggleGroup(group: string) {
    setCollapsedGroups((current) => {
      const next = new Set(current);
      if (next.has(group)) next.delete(group); else next.add(group);
      return next;
    });
  }

  if (collapsed) {
    return <aside className="action-library collapsed" data-layout-region="actions" aria-label="Actions"><button className="panel-expand" aria-label="Expand action panel" title="Expand Actions" onClick={onToggleCollapsed}>‹</button></aside>;
  }

  return <aside className="action-library" data-layout-region="actions" aria-label="Actions">
    <div className="action-library-header">
      <input aria-label="Search actions" placeholder="Search actions" value={query} onChange={(event) => setQuery(event.target.value)} />
      <button className="panel-collapse" aria-label="Collapse action panel" title="Collapse Actions" onClick={onToggleCollapsed}>›</button>
    </div>
    <div className="action-mode-tabs" role="tablist" aria-label="Action modes">
      <button role="tab" aria-selected={mode === 'keys'} className={mode === 'keys' ? 'active' : ''} onClick={() => onModeChange('keys')}>Keys</button>
      <button role="tab" aria-selected={mode === 'dials'} className={mode === 'dials' ? 'active' : ''} onClick={() => onModeChange('dials')}>Dials</button>
    </div>
    <div className="action-scroll">{Array.from(groups.entries()).map(([group, items]) => {
      const groupCollapsed = collapsedGroups.has(group);
      return <section className="action-group" key={group}>
        <button className="action-group-heading" aria-expanded={!groupCollapsed} onClick={() => toggleGroup(group)}><span>{groupCollapsed ? '›' : '⌄'}</span>{group}</button>
        {!groupCollapsed && items.map((item) => <button
          key={item.id}
          draggable
          className="action-entry"
          title={item.label}
          onDragStart={(event) => {
            event.dataTransfer.effectAllowed = 'copy';
            event.dataTransfer.setData(ACTION_MIME, item.id);
          }}
          onClick={() => onChoose(item.id)}
        >{item.label}</button>)}
      </section>;
    })}</div>
  </aside>;
}
