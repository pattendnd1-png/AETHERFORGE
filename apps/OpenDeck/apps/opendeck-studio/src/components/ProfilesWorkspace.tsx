import { memo, useEffect, useMemo, useState } from 'react';
import type { Profile, Workspace } from '../model/workspace';

interface Props {
  workspace: Workspace;
  onActivate: (profileId: string) => void;
  onCreate: () => void;
  onRename: (profileId: string, name: string) => void;
  onDuplicate: (profileId: string) => void;
  onDelete: (profileId: string) => void;
  onExport: (profile: Profile, path: string) => Promise<void>;
  onImport: (path: string) => Promise<void>;
}

export const ProfilesWorkspace = memo(function ProfilesWorkspace({ workspace, onActivate, onCreate, onRename, onDuplicate, onDelete, onExport, onImport }: Props) {
  const [selectedId, setSelectedId] = useState(workspace.active_profile_id);
  const [path, setPath] = useState('');
  const [message, setMessage] = useState('');
  useEffect(() => {
    if (!workspace.profiles.some((profile) => profile.id === selectedId)) setSelectedId(workspace.active_profile_id);
  }, [selectedId, workspace.active_profile_id, workspace.profiles]);
  const selected = useMemo(() => workspace.profiles.find((profile) => profile.id === selectedId) ?? workspace.profiles[0], [selectedId, workspace.profiles]);
  const active = workspace.active_profile_id;

  const rename = () => {
    if (!selected) return;
    const next = window.prompt('Profile name', selected.name)?.trim();
    if (next) onRename(selected.id, next);
  };
  const doExport = async () => {
    if (!selected || !path.trim()) { setMessage('Enter an export path first.'); return; }
    try { await onExport(selected, path.trim()); setMessage(`Exported ${selected.name}.`); } catch (error) { setMessage(String(error)); }
  };
  const doImport = async () => {
    if (!path.trim()) { setMessage('Enter a profile path first.'); return; }
    try { await onImport(path.trim()); setMessage('Profile imported.'); } catch (error) { setMessage(String(error)); }
  };

  return <section className="section-workspace profiles-workspace" aria-label="Profiles">
    <header className="section-workspace-header">
      <div><strong>Profiles</strong><span>Select a profile, then activate it on the Stream Deck.</span></div>
      <button onClick={onCreate}>New Profile</button>
    </header>
    <div className="profile-management-grid">
      <div className="profile-list" role="listbox" aria-label="Profiles">
        {workspace.profiles.map((profile) => <button
          key={profile.id}
          className={`profile-card${selectedId === profile.id ? ' selected' : ''}${active === profile.id ? ' active' : ''}`}
          aria-selected={selectedId === profile.id}
          onClick={() => setSelectedId(profile.id)}
        >
          <span className="profile-card-icon">P</span>
          <span><strong>{profile.name}</strong><small>{profile.pages.length} page{profile.pages.length === 1 ? '' : 's'} · {profile.device_id}{profile.pluginOwnerUuid ? ' · plugin profile' : ''}</small></span>
          {active === profile.id && <em>ACTIVE</em>}
        </button>)}
      </div>
      {selected && <aside className="profile-details">
        <div><small>Selected profile</small><h2>{selected.name}</h2><p>{selected.pages.length} page{selected.pages.length === 1 ? '' : 's'} configured for {selected.device_id}.{selected.pluginReadonly ? ' This plugin profile is read-only.' : ''}</p></div>
        <div className="profile-action-row">
          <button className="primary" disabled={selected.id === active} onClick={() => onActivate(selected.id)}>{selected.id === active ? 'Active' : 'Activate'}</button>
          <button disabled={selected.pluginReadonly} onClick={rename}>Rename</button>
          <button onClick={() => onDuplicate(selected.id)}>Duplicate</button>
          <button disabled={workspace.profiles.length <= 1 || selected.pluginReadonly} onClick={() => onDelete(selected.id)}>Delete</button>
        </div>
        <div className="profile-io">
          <label><span>Import / export path</span><input value={path} onChange={(event) => setPath(event.target.value)} placeholder="/home/you/Downloads/profile.streamDeckProfile" /></label>
          <div className="profile-action-row"><button onClick={() => void doExport()}>Export selected</button><button onClick={() => void doImport()}>Import profile</button></div>
          {message && <p role="status">{message}</p>}
        </div>
      </aside>}
    </div>
  </section>;
});
