import { useEffect, useState } from 'react';
import type { Profile } from '../model/workspace';

interface PageNavigatorProps {
  profile: Profile;
  onSelect: (id: string) => void;
  onAdd: () => void;
  onDuplicate: () => void;
  onRename: (id: string, name: string) => void;
  onDelete: () => void;
}

export function PageNavigator({ profile, onSelect, onAdd, onDuplicate, onRename, onDelete }: PageNavigatorProps) {
  const index = Math.max(0, profile.pages.findIndex((page) => page.id === profile.active_page_id));
  const page = profile.pages[index];
  const [editing, setEditing] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);
  const [name, setName] = useState(page.name);

  useEffect(() => {
    setName(page.name);
    setEditing(false);
    setMenuOpen(false);
  }, [page.id, page.name]);

  function saveRename() {
    const next = name.trim();
    if (next) onRename(page.id, next);
    setEditing(false);
  }

  return <div className="page-navigator" aria-label="Pages">
    <span className="pages-label">Pages:</span>
    {profile.pages.map((candidate, i) => <button
      key={candidate.id}
      className="page-number"
      aria-label={`Page ${i + 1}`}
      aria-pressed={candidate.id === page.id}
      onClick={() => onSelect(candidate.id)}
    >{i + 1}</button>)}
    <button className="page-add" aria-label="Add page" title="Add page" onClick={onAdd}>＋</button>
    <button className="page-options" aria-label="Page options" title="Page options" onClick={() => setMenuOpen((open) => !open)}>⋯</button>
    {menuOpen && <div className="inline-popover page-options-popover">
      <button onClick={() => { setName(page.name); setMenuOpen(false); setEditing(true); }}>Rename page</button>
      <button onClick={() => { setMenuOpen(false); onDuplicate(); }}>Duplicate page</button>
      <button disabled={profile.pages.length <= 1} onClick={() => { setMenuOpen(false); onDelete(); }}>Delete page</button>
    </div>}
    {editing && <div className="inline-popover page-rename-popover">
      <input autoFocus aria-label="Page name" value={name} onChange={(event) => setName(event.target.value)} onKeyDown={(event) => { if (event.key === 'Enter') saveRename(); if (event.key === 'Escape') setEditing(false); }} />
      <button onClick={saveRename}>Save</button>
    </div>}
  </div>;
}
