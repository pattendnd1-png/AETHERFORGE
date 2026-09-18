import { memo, useMemo, useState } from 'react';
import type { MarketplaceItem } from '../types';
import type { PluginDescriptor, PluginHostStatus } from '../plugins/types';

interface Props {
  plugins: PluginDescriptor[];
  hostStatus: PluginHostStatus | null;
  marketplaceItems: MarketplaceItem[];
  onScan: () => Promise<void>;
  onOpenMarketplace: () => Promise<void>;
  onInstall: (path: string) => Promise<void>;
  onSetEnabled: (uuid: string, enabled: boolean) => Promise<void>;
  onSetActive: (uuid: string, active: boolean) => Promise<void>;
  onRestart: (uuid: string) => Promise<void>;
  onRemove: (uuid: string) => Promise<void>;
  onImportBundledProfile: (uuid: string, profileName: string, activate: boolean) => Promise<void>;
}

export const PluginManager = memo(function PluginManager({ plugins, hostStatus, marketplaceItems, onScan, onOpenMarketplace, onInstall, onSetEnabled, onSetActive, onRestart, onRemove, onImportBundledProfile }: Props) {
  const [selectedUuid, setSelectedUuid] = useState<string | null>(plugins[0]?.uuid ?? null);
  const [manualPath, setManualPath] = useState('');
  const [message, setMessage] = useState('');
  const selected = useMemo(() => plugins.find((plugin) => plugin.uuid === selectedUuid) ?? plugins[0] ?? null, [plugins, selectedUuid]);

  async function run(label: string, action: () => Promise<void>) {
    setMessage(`${label}…`);
    try { await action(); setMessage(`${label}: done.`); } catch (error) { setMessage(`${label}: ${String(error)}`); }
  }

  const packages = marketplaceItems.filter((item) => item.kind === 'plugin');
  return <section className="section-workspace plugins-workspace" aria-label="Plugins">
    <header className="section-workspace-header">
      <div><strong>Plugins</strong><span>OpenDeck-native and Stream Deck-compatible plugin host.</span></div>
      <div className="plugin-host-summary"><span>{hostStatus?.active ?? 0} active</span><span>{hostStatus?.installed ?? plugins.length} installed</span><span>SDK target {hostStatus?.streamDeckCompatibilityTarget ?? '7.6'}</span></div>
    </header>
    <div className="plugin-management-grid">
      <div className="plugin-list" role="listbox" aria-label="Installed plugins">
        {plugins.length === 0 && <p className="empty-state">No plugins installed yet.</p>}
        {plugins.map((plugin) => <button key={plugin.uuid} className={`plugin-card${selected?.uuid === plugin.uuid ? ' selected' : ''}`} aria-selected={selected?.uuid === plugin.uuid} onClick={() => setSelectedUuid(plugin.uuid)}>
          <span className={`plugin-state-dot ${plugin.processState}`} />
          <span><strong>{plugin.name}</strong><small>{plugin.version} · {plugin.sourceKind === 'elgato' ? 'Stream Deck' : 'OpenDeck'} · {plugin.compatibility}</small></span>
          <em>{plugin.active ? 'ACTIVE' : plugin.enabled ? 'ENABLED' : 'DISABLED'}</em>
        </button>)}
      </div>
      <aside className="plugin-details">
        {selected ? <>
          <div><small>{selected.uuid}</small><h2>{selected.name}</h2><p>{selected.description || 'No description provided.'}</p></div>
          <dl className="plugin-meta"><div><dt>Runtime</dt><dd>{selected.runtimeKind ?? 'unavailable'}</dd></div><div><dt>Process</dt><dd>{selected.processState}</dd></div><div><dt>SDK</dt><dd>{selected.sdkVersion}</dd></div><div><dt>Actions</dt><dd>{selected.actions.length}</dd></div></dl>
          {selected.lastError && <p className="plugin-error" role="alert">{selected.lastError}</p>}
          <div className="profile-action-row">
            <button className="primary" disabled={!selected.enabled || selected.compatibility === 'drm-protected' || selected.compatibility === 'unsupported'} onClick={() => void run(selected.active ? 'Deactivating' : 'Activating', () => onSetActive(selected.uuid, !selected.active))}>{selected.active ? 'Deactivate' : 'Activate'}</button>
            <button onClick={() => void run(selected.enabled ? 'Disabling' : 'Enabling', () => onSetEnabled(selected.uuid, !selected.enabled))}>{selected.enabled ? 'Disable' : 'Enable'}</button>
            <button disabled={!selected.active} onClick={() => void run('Restarting', () => onRestart(selected.uuid))}>Restart</button>
            <button onClick={() => void run('Removing', () => onRemove(selected.uuid))}>Remove</button>
          </div>
          <div className="plugin-action-list"><h3>Registered actions</h3>{selected.actions.map((action) => <div key={action.uuid}><strong>{action.name}</strong><small>{action.controllers.join(', ')}{action.propertyInspectorPath ? ' · Property Inspector' : ''}</small></div>)}</div>
          {selected.profiles.length > 0 && <div className="plugin-profile-list"><h3>Bundled profiles</h3>{selected.profiles.map((profile) => <div key={`${selected.uuid}:${profile.name}`}><span><strong>{profile.name}</strong><small>Device {profile.deviceType}{profile.readonly ? ' · read-only' : ''}{profile.autoInstall ? ' · auto-install' : ''}</small></span><button disabled={profile.deviceType !== 7} onClick={() => void run(`Installing profile ${profile.name}`, () => onImportBundledProfile(selected.uuid, profile.name, !profile.dontAutoSwitchWhenInstalled))}>{profile.deviceType === 7 ? 'Install' : 'Other device'}</button></div>)}</div>}
        </> : <p className="empty-state">Select an installed plugin.</p>}
        <div className="plugin-install-panel">
          <h3>Install plugin</h3>
          <label><span>Package or plugin directory</span><input value={manualPath} onChange={(event) => setManualPath(event.target.value)} placeholder="~/Downloads/example.streamDeckPlugin" /></label>
          <div className="profile-action-row"><button disabled={!manualPath.trim()} onClick={() => void run('Installing', () => onInstall(manualPath.trim()))}>Install path</button><button onClick={() => void run('Scanning Downloads', onScan)}>Scan Downloads</button><button onClick={() => void onOpenMarketplace()}>Marketplace</button></div>
          {packages.length > 0 && <div className="downloaded-plugin-list">{packages.map((item) => <div key={item.path}><span><strong>{item.name}</strong><small>{item.path}</small></span><button onClick={() => void run(`Installing ${item.name}`, () => onInstall(item.path))}>Install</button></div>)}</div>}
          {message && <p role="status">{message}</p>}
        </div>
      </aside>
    </div>
  </section>;
});
