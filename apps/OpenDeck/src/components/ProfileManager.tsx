import { useEffect, useState } from 'react';
import type { Workspace } from '../model/workspace';

interface Props {
  workspace: Workspace;
  onSelect: (profileId: string) => void;
  onCreate: () => void;
  onRename: (profileId: string, name: string) => void;
  onDuplicate: (profileId: string) => void;
  onDelete: (profileId: string) => void;
}

export function ProfileManager({ workspace, onSelect, onCreate, onRename, onDuplicate, onDelete }: Props) {
  const [editing, setEditing] = useState(false);
  const [menuOpen, setMenuOpen] = useState(false);
  const active = workspace.profiles.find((profile) => profile.id === workspace.active_profile_id) ?? workspace.profiles[0];
  const [name, setName] = useState(active.name);

  useEffect(() => {
    setName(active.name);
    setEditing(false);
    setMenuOpen(false);
  }, [active.id, active.name]);

  function saveRename() {
    const next = name.trim();
    if (next) onRename(active.id, next);
    setEditing(false);
  }

  return <div className="profile-manager">
    <select aria-label="Profile" value={active.id} onChange={(event) => onSelect(event.target.value)}>
      {workspace.profiles.map((profile) => <option key={profile.id} value={profile.id}>{profile.name}</option>)}
    </select>
    <button aria-label="Profile options" title="Profile options" onClick={() => setMenuOpen((open) => !open)}>⋯</button>
    {menuOpen && <div className="inline-popover profile-options-popover">
      <button onClick={() => { setMenuOpen(false); onCreate(); }}>New profile</button>
      <button onClick={() => { setName(active.name); setMenuOpen(false); setEditing(true); }}>Rename profile</button>
      <button onClick={() => { setMenuOpen(false); onDuplicate(active.id); }}>Duplicate profile</button>
      <button disabled={workspace.profiles.length <= 1} onClick={() => { setMenuOpen(false); onDelete(active.id); }}>Delete profile</button>
    </div>}
    {editing && <div className="inline-popover profile-rename-popover">
      <input autoFocus aria-label="Profile name" value={name} onChange={(event) => setName(event.target.value)} onKeyDown={(event) => { if (event.key === 'Enter') saveRename(); if (event.key === 'Escape') setEditing(false); }} />
      <button onClick={saveRename}>Save</button>
    </div>}
  </div>;
}
