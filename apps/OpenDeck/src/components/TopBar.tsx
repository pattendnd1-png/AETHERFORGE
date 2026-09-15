import { ProfileManager } from './ProfileManager';
import type { Profile, Workspace } from '../model/workspace';

interface TopBarProps {
  workspace: Workspace;
  profile: Profile;
  canUndo: boolean;
  canRedo: boolean;
  onUndo: () => void;
  onRedo: () => void;
  onConnections: () => void;
  onMarketplace: () => void;
  onSelectProfile: (id: string) => void;
  onCreateProfile: () => void;
  onRenameProfile: (id: string, name: string) => void;
  onDuplicateProfile: (id: string) => void;
  onDeleteProfile: (id: string) => void;
}

export function TopBar({ workspace, profile, canUndo, canRedo, onUndo, onRedo, onConnections, onMarketplace, onSelectProfile, onCreateProfile, onRenameProfile, onDuplicateProfile, onDeleteProfile }: TopBarProps) {
  return <header className="topbar compact-topbar">
    <div className="selectors">
      <label><span>Device</span><select aria-label="Device" value="Stream Deck +" onChange={() => undefined}><option>Stream Deck +</option></select></label>
      <label><span>Profile</span><ProfileManager workspace={workspace} onSelect={onSelectProfile} onCreate={onCreateProfile} onRename={onRenameProfile} onDuplicate={onDuplicateProfile} onDelete={onDeleteProfile} /></label>
    </div>
    <div className="top-actions">
      <button disabled={!canUndo} onClick={onUndo} title="Undo">↶</button>
      <button disabled={!canRedo} onClick={onRedo} title="Redo">↷</button>
      <button onClick={onConnections}>Connections</button>
      <button onClick={onMarketplace}>Marketplace</button>
    </div>
  </header>;
}
