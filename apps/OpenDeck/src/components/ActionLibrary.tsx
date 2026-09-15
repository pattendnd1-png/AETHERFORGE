import { useMemo, useState } from 'react';
import { ACTION_DEFINITIONS, supportsControl } from '../model/actions';
import type { ControlKind } from '../model/workspace';

const ACTION_MIME = 'application/x-opendeck-action';

interface ActionLibraryProps {
  selectedKind?: ControlKind;
  collapsed: boolean;
  onChoose: (id: string) => void;
  onToggleCollapsed: () => void;
}

export function ActionLibrary({ selectedKind, collapsed, onChoose, onToggleCollapsed }: ActionLibraryProps) {
  const [query, setQuery] = useState('');
  const groups = useMemo(() => {
    const filtered = ACTION_DEFINITIONS.filter((item) => `${item.group} ${item.label}`.toLowerCase().includes(query.toLowerCase()));
    const grouped = new Map<string, typeof ACTION_DEFINITIONS>();
    for (const item of filtered) {
      const existing = grouped.get(item.group) ?? [];
      grouped.set(item.group, [...existing, item]);
    }
    return grouped;
  }, [query]);

  if (collapsed) {
    return <aside className="action-library collapsed"><button className="panel-expand" aria-label="Expand action panel" title="Expand Actions" onClick={onToggleCollapsed}>‹</button></aside>;
  }

  return <aside className="action-library">
    <div className="panel-heading"><strong>Actions</strong><input aria-label="Search actions" placeholder="Search" value={query} onChange={(event) => setQuery(event.target.value)} /><button className="panel-collapse" aria-label="Collapse action panel" title="Collapse Actions" onClick={onToggleCollapsed}>›</button></div>
    <div className="action-scroll">{Array.from(groups.entries()).map(([group, items]) => <section className="action-group" key={group}><h3>{group}</h3>{items.map((item) => {
      const unsupported = selectedKind ? !supportsControl(item.id, selectedKind) : false;
      return <button
        key={item.id}
        draggable
        className={unsupported ? 'unsupported' : ''}
        title={unsupported ? `Not supported on selected ${selectedKind}; drag to a supported control` : item.label}
        onDragStart={(event) => {
          event.dataTransfer.effectAllowed = 'copy';
          event.dataTransfer.setData(ACTION_MIME, item.id);
        }}
        onClick={() => onChoose(item.id)}
      >{item.label}</button>;
    })}</section>)}</div>
  </aside>;
}
