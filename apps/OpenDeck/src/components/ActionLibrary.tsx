import { useMemo, useState } from 'react';
import { ACTION_DEFINITIONS, supportsControl } from '../model/actions';
import type { ControlKind } from '../model/workspace';

interface ActionLibraryProps { selectedKind?: ControlKind; onChoose: (id: string) => void }

export function ActionLibrary({ selectedKind, onChoose }: ActionLibraryProps) {
  const [query, setQuery] = useState('');
  const groups = useMemo(() => {
    const filtered = ACTION_DEFINITIONS.filter((item) => `${item.group} ${item.label}`.toLowerCase().includes(query.toLowerCase()));
    return Map.groupBy(filtered, (item) => item.group);
  }, [query]);
  return <aside className="action-library">
    <div className="panel-heading"><strong>Actions</strong><input aria-label="Search actions" placeholder="Search" value={query} onChange={(event) => setQuery(event.target.value)} /></div>
    <div className="action-scroll">{Array.from(groups.entries()).map(([group, items]) => <section className="action-group" key={group}><h3>{group}</h3>{items.map((item) => {
      const disabled = selectedKind ? !supportsControl(item.id, selectedKind) : false;
      return <button key={item.id} disabled={disabled} title={disabled ? `Not supported on ${selectedKind}` : item.label} onClick={() => onChoose(item.id)}>{item.label}</button>;
    })}</section>)}</div>
  </aside>;
}
