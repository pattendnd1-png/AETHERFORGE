import { ProfileManager } from './ProfileManager';
import { Glyph } from './Glyph';
import { bridge } from '../bridge';
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
  onSettings: () => void;
  onSelectProfile: (id: string) => void;
  onCreateProfile: () => void;
  onRenameProfile: (id: string, name: string) => void;
  onDuplicateProfile: (id: string) => void;
  onDeleteProfile: (id: string) => void;
}

export function TopBar({ workspace, hardwareStatus, saveStatus, canUndo, canRedo, onUndo, onRedo, onConnections, onMarketplace, onSettings, onSelectProfile, onCreateProfile, onRenameProfile, onDuplicateProfile, onDeleteProfile }: TopBarProps) {
  const connected = hardwareStatus.state === 'connected';
  return <header className="topbar" data-layout-region="header" onMouseDown={(event) => { if (event.button === 0 && event.target === event.currentTarget) void bridge.appStartDragging(); }}>
    <div className="brand-block">
      <span className="brand-mark"><span /></span>
      <div><div className="brand-title">OpenDeck+ <span>2.0.28</span></div><div className="brand-subtitle">Control More. Create Freely.</div></div>
    </div>
    <div className="header-selectors">
      <label className="header-selector"><span>Device</span><div className="select-shell"><Glyph name="device"/><select aria-label="Device" value="Stream Deck +" onChange={() => undefined}><option>Stream Deck +</option></select></div></label>
      <div className={`connection-state ${connected ? 'connected' : ''}`}><span className="hardware-status-dot" title={hardwareStatus.message} />{connected ? 'Connected' : hardwareStatus.state}</div>
      <label className="header-selector profile-selector"><span>Profile</span><div className="select-shell"><Glyph name="profile"/><ProfileManager workspace={workspace} onSelect={onSelectProfile} onCreate={onCreateProfile} onRename={onRenameProfile} onDuplicate={onDuplicateProfile} onDelete={onDeleteProfile} /></div></label>
    </div>
    <div className="app-controls">
      <span className={`save-state ${saveStatus.toLowerCase()}`} aria-label={`Save status: ${saveStatus}`}>✓ {saveStatus}</span>
      <button className="header-icon-button" disabled={!canUndo} onClick={onUndo} aria-label="Undo" title="Undo"><Glyph name="undo" /></button>
      <button className="header-icon-button" disabled={!canRedo} onClick={onRedo} aria-label="Redo" title="Redo"><Glyph name="redo" /></button>
      <button className="header-icon-button secondary-tool" onClick={onConnections} aria-label="Connections" title="Connections"><Glyph name="link" /></button>
      <button className="header-icon-button secondary-tool" onClick={onMarketplace} aria-label="Marketplace" title="Marketplace"><Glyph name="market" /></button>
      <button className="header-icon-button" onClick={onSettings} aria-label="Settings" title="Settings"><Glyph name="settings" /></button>
      <span className="window-control-divider" aria-hidden="true" />
      <button className="window-control" onClick={() => void bridge.appMinimize()} aria-label="Minimize window" title="Minimize">−</button>
      <button className="window-control" onClick={() => void bridge.appToggleMaximize()} aria-label="Maximize window" title="Maximize">□</button>
      <button className="window-control close-window" onClick={() => void bridge.appClose()} aria-label="Close window" title="Close">×</button>
    </div>
  </header>;
}
