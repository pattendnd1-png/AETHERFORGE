import { ProfileManager } from './ProfileManager';
import type { Workspace } from '../model/workspace';
import type { StreamDeckStatus } from '../types';

interface TopBarProps {
  workspace: Workspace;
  hardwareStatus: StreamDeckStatus;
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

export function TopBar({ workspace, hardwareStatus, canUndo, canRedo, onUndo, onRedo, onConnections, onMarketplace, onSelectProfile, onCreateProfile, onRenameProfile, onDuplicateProfile, onDeleteProfile }: TopBarProps) {
  return <header className="topbar compact-topbar">
    <div className="selectors">
      <label><span>Device</span><select aria-label="Device" value="Stream Deck +" onChange={() => undefined}><option>Stream Deck +</option></select></label>
      <span className={`hardware-status ${hardwareStatus.state}`} title={hardwareStatus.message}>
        <span className="hardware-status-dot" aria-hidden="true" />
        Stream Deck + · {hardwareStatus.state === 'connected' ? 'Connected' : hardwareStatus.state === 'permissionDenied' ? 'Permission' : hardwareStatus.state === 'ioError' ? 'Error' : hardwareStatus.state === 'connecting' ? 'Connecting' : 'Disconnected'}
      </span>
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
