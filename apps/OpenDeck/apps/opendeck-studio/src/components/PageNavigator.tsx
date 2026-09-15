import { useState } from 'react';
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
  const [name, setName] = useState(page.name);

  function saveRename() {
    const next = name.trim();
    if (next) onRename(page.id, next);
    setEditing(false);
  }

  return <div className="page-navigator">
    <button disabled={index <= 0} aria-label="Previous page" onClick={() => onSelect(profile.pages[index - 1]?.id ?? page.id)}>‹</button>
    <select aria-label="Page" value={page.id} onChange={(event) => onSelect(event.target.value)}>{profile.pages.map((candidate, i) => <option key={candidate.id} value={candidate.id}>{i + 1}. {candidate.name}</option>)}</select>
    <button disabled={index >= profile.pages.length - 1} aria-label="Next page" onClick={() => onSelect(profile.pages[index + 1]?.id ?? page.id)}>›</button>
    <span className="nav-separator" />
    <button title="Add page" aria-label="Add page" onClick={onAdd}>＋</button>
    <button title="Rename page" aria-label="Rename page" onClick={() => { setName(page.name); setEditing(true); }}>✎</button>
    <button title="Duplicate page" aria-label="Duplicate page" onClick={onDuplicate}>⧉</button>
    <button title="Delete page" aria-label="Delete page" disabled={profile.pages.length <= 1} onClick={onDelete}>−</button>
    {editing && <div className="inline-popover page-rename-popover"><input autoFocus aria-label="Page name" value={name} onChange={(event) => setName(event.target.value)} onKeyDown={(event) => { if (event.key === 'Enter') saveRename(); if (event.key === 'Escape') setEditing(false); }} /><button onClick={saveRename}>Save</button></div>}
  </div>;
}
