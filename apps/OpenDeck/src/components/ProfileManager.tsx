import { useState } from 'react';
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
  const active = workspace.profiles.find((profile) => profile.id === workspace.active_profile_id) ?? workspace.profiles[0];
  const [name, setName] = useState(active.name);
  return <div className="profile-manager">
    <select aria-label="Profile" value={active.id} onChange={(e) => { onSelect(e.target.value); const next = workspace.profiles.find((profile) => profile.id === e.target.value); if (next) setName(next.name); }}>
      {workspace.profiles.map((profile) => <option key={profile.id} value={profile.id}>{profile.name}</option>)}
    </select>
    <button title="New profile" onClick={onCreate}>＋</button>
    <button title="Rename profile" onClick={() => { setName(active.name); setEditing(true); }}>✎</button>
    <button title="Duplicate profile" onClick={() => onDuplicate(active.id)}>⧉</button>
    <button title="Delete profile" disabled={workspace.profiles.length <= 1} onClick={() => onDelete(active.id)}>−</button>
    {editing && <div className="inline-popover"><input autoFocus aria-label="Profile name" value={name} onChange={(e) => setName(e.target.value)} onKeyDown={(e) => { if (e.key === 'Enter' && name.trim()) { onRename(active.id, name.trim()); setEditing(false); } if (e.key === 'Escape') setEditing(false); }} /><button onClick={() => { if (name.trim()) onRename(active.id, name.trim()); setEditing(false); }}>Save</button></div>}
  </div>;
}
