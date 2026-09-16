import { ProfileManager } from './ProfileManager';
import type { Workspace } from '../model/workspace';
import type { StreamDeckStatus } from '../types';

interface TopBarProps {
  workspace: Workspace;
  hardwareStatus: StreamDeckStatus;
  saveStatus: 'Loading'|'Saved'|'Dirty'|'Saving'|'Error';
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

export function TopBar({ workspace, hardwareStatus, saveStatus, canUndo, canRedo, onUndo, onRedo, onConnections, onMarketplace, onSelectProfile, onCreateProfile, onRenameProfile, onDuplicateProfile, onDeleteProfile }: TopBarProps) {
  return <header className="topbar compact-topbar">
    <div className="editor-identity">
      <div className="device-row">
        <label className="selector-field"><span>Device</span><select aria-label="Device" value="Stream Deck +" onChange={() => undefined}><option>Stream Deck +</option></select></label>
        <span className={`hardware-status-dot ${hardwareStatus.state}`} title={hardwareStatus.message} aria-label={`Stream Deck +: ${hardwareStatus.message}`} />
      </div>
      <div className="profile-row">
        <span className="field-label">Profile</span>
        <ProfileManager workspace={workspace} onSelect={onSelectProfile} onCreate={onCreateProfile} onRename={onRenameProfile} onDuplicate={onDuplicateProfile} onDelete={onDeleteProfile} />
      </div>
    </div>
    <div className="app-controls">
      <span className={`save-state ${saveStatus.toLowerCase()}`} aria-label={`Save status: ${saveStatus}`}>{saveStatus}</span>
      <button className="icon-button" disabled={!canUndo} onClick={onUndo} aria-label="Undo" title="Undo">↶</button>
      <button className="icon-button" disabled={!canRedo} onClick={onRedo} aria-label="Redo" title="Redo">↷</button>
      <button className="icon-button" onClick={onConnections} aria-label="Connections" title="Connections">⛓</button>
      <button className="icon-button" onClick={onMarketplace} aria-label="Marketplace" title="Marketplace">◇</button>
    </div>
  </header>;
}
