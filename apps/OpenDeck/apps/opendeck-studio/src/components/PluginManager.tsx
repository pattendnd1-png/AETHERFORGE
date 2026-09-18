import { memo, useEffect, useMemo, useRef, useState } from 'react';
import type { IconPackDescriptor, MarketplaceItem } from '../types';
import type { PluginDescriptor, PluginHostStatus } from '../plugins/types';

type Selection =
  | { kind: 'plugin'; id: string }
  | { kind: 'icon-pack'; id: string }
  | { kind: 'download'; id: string };

interface Props {
  plugins: PluginDescriptor[];
  iconPacks: IconPackDescriptor[];
  hostStatus: PluginHostStatus | null;
  marketplaceItems: MarketplaceItem[];
  onScan: () => Promise<void>;
  onOpenMarketplace: () => Promise<void>;
  onInstallPackage: (path: string, activate?: boolean) => Promise<void>;
  onSetEnabled: (uuid: string, enabled: boolean) => Promise<void>;
  onSetActive: (uuid: string, active: boolean) => Promise<void>;
  onRestart: (uuid: string) => Promise<void>;
  onRemove: (uuid: string) => Promise<void>;
  onSetIconPackActive: (packId: string, active: boolean) => Promise<void>;
  onRemoveIconPack: (packId: string) => Promise<void>;
  onImportBundledProfile: (uuid: string, profileName: string, activate: boolean) => Promise<void>;
}

const SUPPORTED_DOWNLOAD_KINDS = new Set(['plugin', 'icon_pack', 'profile']);

function itemKey(item: MarketplaceItem): string {
  return `${item.kind}:${item.path}`;
}

function kindLabel(kind: string): string {
  if (kind === 'plugin') return 'Plugin';
  if (kind === 'icon_pack') return 'Icon Pack';
  if (kind === 'profile') return 'Profile Pack';
  return kind.replaceAll('_', ' ');
}

export const PluginManager = memo(function PluginManager({
  plugins,
  iconPacks,
  hostStatus,
  marketplaceItems,
  onScan,
  onOpenMarketplace,
  onInstallPackage,
  onSetEnabled,
  onSetActive,
  onRestart,
  onRemove,
  onSetIconPackActive,
  onRemoveIconPack,
  onImportBundledProfile,
}: Props) {
  const downloads = useMemo(
    () => marketplaceItems.filter((item) => SUPPORTED_DOWNLOAD_KINDS.has(item.kind)),
    [marketplaceItems],
  );
  const [selection, setSelection] = useState<Selection | null>(null);
  const [manualPath, setManualPath] = useState('');
  const [message, setMessage] = useState('');
  const [packageErrors, setPackageErrors] = useState<Record<string, string>>({});
  const [busyLabel, setBusyLabel] = useState<string | null>(null);
  const busyRef = useRef(false);
  const busy = busyLabel !== null;

  useEffect(() => {
    if (selection?.kind === 'plugin' && plugins.some((plugin) => plugin.uuid === selection.id)) return;
    if (selection?.kind === 'icon-pack' && iconPacks.some((pack) => pack.id === selection.id)) return;
    if (selection?.kind === 'download' && downloads.some((item) => itemKey(item) === selection.id)) return;
    if (plugins[0]) setSelection({ kind: 'plugin', id: plugins[0].uuid });
    else if (iconPacks[0]) setSelection({ kind: 'icon-pack', id: iconPacks[0].id });
    else if (downloads[0]) setSelection({ kind: 'download', id: itemKey(downloads[0]) });
    else setSelection(null);
  }, [downloads, iconPacks, plugins, selection]);

  const selectedPlugin = selection?.kind === 'plugin'
    ? plugins.find((plugin) => plugin.uuid === selection.id) ?? null
    : null;
  const selectedPack = selection?.kind === 'icon-pack'
    ? iconPacks.find((pack) => pack.id === selection.id) ?? null
    : null;
  const selectedDownload = selection?.kind === 'download'
    ? downloads.find((item) => itemKey(item) === selection.id) ?? null
    : null;
  const manualNormalized = manualPath.trim().toLowerCase();
  const manualIsPlugin = manualNormalized.endsWith('.streamdeckplugin') || manualNormalized.endsWith('.sdplugin');

  async function run(label: string, action: () => Promise<void>) {
    if (busyRef.current) return;
    busyRef.current = true;
    setBusyLabel(label);
    setMessage(`${label}…`);
    try {
      await action();
      setMessage(`${label}: finished. Check the installed status above.`);
    } catch (error) {
      setMessage(`${label}: ${String(error)}`);
    } finally {
      busyRef.current = false;
      setBusyLabel(null);
    }
  }

  async function installDownloaded(item: MarketplaceItem, activate = true) {
    if (busyRef.current) return;
    busyRef.current = true;
    const key = itemKey(item);
    const label = item.kind === 'profile'
      ? 'Importing and activating profile'
      : activate ? 'Installing and activating' : 'Installing';
    setBusyLabel(label);
    setMessage(`${label}…`);
    setPackageErrors((current) => {
      const next = { ...current };
      delete next[key];
      return next;
    });
    try {
      await onInstallPackage(item.path, activate);
      setMessage(`${label}: done.`);
    } catch (error) {
      const text = String(error);
      setPackageErrors((current) => ({ ...current, [key]: text }));
      setMessage(`${label}: ${text}`);
    } finally {
      busyRef.current = false;
      setBusyLabel(null);
    }
  }

  return <section className="section-workspace plugins-workspace" aria-label="Plugins and Packs" aria-busy={busy}>
    <header className="section-workspace-header">
      <div><strong>Plugins &amp; Packs</strong><span>Select, install, activate, deactivate, and remove OpenDeck / Stream Deck extensions.</span></div>
      <div className="plugin-host-summary">
        {busyLabel && <span className="extension-busy">{busyLabel}…</span>}
        <span>{hostStatus?.active ?? 0} plugins active</span>
        <span>{hostStatus?.installed ?? plugins.length} plugins installed</span>
        <span>{iconPacks.filter((pack) => pack.active).length} packs active</span>
        <span>SDK target {hostStatus?.streamDeckCompatibilityTarget ?? '7.6'}</span>
      </div>
    </header>

    <div className="plugin-management-grid">
      <div className="plugin-list extension-list" role="listbox" aria-label="Installed plugins and packs">
        <h3>Installed Plugins</h3>
        {plugins.length === 0 && <p className="empty-state">No plugins installed yet.</p>}
        {plugins.map((plugin) => <button
          type="button"
          key={plugin.uuid}
          className={`plugin-card${selectedPlugin?.uuid === plugin.uuid ? ' selected' : ''}`}
          aria-selected={selectedPlugin?.uuid === plugin.uuid}
          disabled={busy}
          onClick={() => setSelection({ kind: 'plugin', id: plugin.uuid })}
        >
          <span className={`plugin-state-dot ${plugin.processState}`} />
          <span><strong>{plugin.name}</strong><small>{plugin.version} · {plugin.sourceKind === 'elgato' ? 'Stream Deck' : 'OpenDeck'} · {plugin.compatibility}</small></span>
          <em>{plugin.active ? 'ACTIVE' : plugin.enabled ? 'ENABLED' : 'DISABLED'}</em>
        </button>)}

        <h3>Installed Icon Packs</h3>
        {iconPacks.length === 0 && <p className="empty-state">No icon packs installed yet.</p>}
        {iconPacks.map((pack) => <button
          type="button"
          key={pack.id}
          className={`plugin-card pack-card${selectedPack?.id === pack.id ? ' selected' : ''}`}
          aria-selected={selectedPack?.id === pack.id}
          disabled={busy}
          onClick={() => setSelection({ kind: 'icon-pack', id: pack.id })}
        >
          <span className={`plugin-state-dot ${pack.active ? 'active' : 'inactive'}`} />
          <span><strong>{pack.name}</strong><small>{pack.version} · {pack.itemCount} icons · {pack.author}</small></span>
          <em>{pack.active ? 'ACTIVE' : 'INACTIVE'}</em>
        </button>)}

        <h3>Downloaded Packages</h3>
        {downloads.length === 0 && <p className="empty-state">Scan Downloads to find Stream Deck plugins, icon packs, and profiles.</p>}
        {downloads.map((item) => {
          const error = packageErrors[itemKey(item)];
          return <button
            type="button"
            key={itemKey(item)}
            className={`plugin-card package-card${selectedDownload && itemKey(selectedDownload) === itemKey(item) ? ' selected' : ''}`}
            aria-selected={Boolean(selectedDownload && itemKey(selectedDownload) === itemKey(item))}
            disabled={busy}
            onClick={() => setSelection({ kind: 'download', id: itemKey(item) })}
          >
            <span className={`plugin-state-dot ${error ? 'error' : 'inactive'}`} />
            <span><strong>{item.name}</strong><small>{kindLabel(item.kind)} · {item.path}</small></span>
            <em>{error ? 'ERROR' : 'DETECTED'}</em>
          </button>;
        })}
      </div>

      <aside className="plugin-details">
        {selectedPlugin ? <>
          <div><small>{selectedPlugin.uuid}</small><h2>{selectedPlugin.name}</h2><p>{selectedPlugin.description || 'No description provided.'}</p></div>
          <dl className="plugin-meta"><div><dt>Runtime</dt><dd>{selectedPlugin.runtimeKind ?? 'unavailable'}</dd></div><div><dt>Process</dt><dd>{selectedPlugin.processState}</dd></div><div><dt>SDK</dt><dd>{selectedPlugin.sdkVersion}</dd></div><div><dt>Actions</dt><dd>{selectedPlugin.actions.length}</dd></div></dl>
          {selectedPlugin.lastError && <p className="plugin-error" role="alert">{selectedPlugin.lastError}</p>}
          <div className="profile-action-row">
            <button
              type="button"
              className="primary"
              disabled={busy || selectedPlugin.compatibility === 'drm-protected' || selectedPlugin.compatibility === 'unsupported'}
              onClick={() => void run(selectedPlugin.active ? 'Deactivating' : 'Activating', () => onSetActive(selectedPlugin.uuid, !selectedPlugin.active))}
            >{selectedPlugin.active ? 'Deactivate Plugin' : 'Activate Plugin'}</button>
            <button type="button" disabled={busy} onClick={() => void run(selectedPlugin.enabled ? 'Disabling' : 'Enabling', () => onSetEnabled(selectedPlugin.uuid, !selectedPlugin.enabled))}>{selectedPlugin.enabled ? 'Disable' : 'Enable'}</button>
            <button type="button" disabled={busy || !selectedPlugin.active} onClick={() => void run('Restarting', () => onRestart(selectedPlugin.uuid))}>Restart</button>
            <button type="button" disabled={busy} onClick={() => void run('Removing', () => onRemove(selectedPlugin.uuid))}>Remove</button>
          </div>
          <div className="plugin-action-list"><h3>Registered actions</h3>{selectedPlugin.actions.map((action) => <div key={action.uuid}><strong>{action.name}</strong><small>{action.controllers.join(', ')}{action.propertyInspectorPath ? ' · Property Inspector' : ''}</small></div>)}</div>
          {selectedPlugin.profiles.length > 0 && <div className="plugin-profile-list"><h3>Bundled profiles</h3>{selectedPlugin.profiles.map((profile) => <div key={`${selectedPlugin.uuid}:${profile.name}`}><span><strong>{profile.name}</strong><small>Device {profile.deviceType}{profile.readonly ? ' · read-only' : ''}{profile.autoInstall ? ' · auto-install' : ''}</small></span><button type="button" disabled={busy || profile.deviceType !== 7} onClick={() => void run(`Installing profile ${profile.name}`, () => onImportBundledProfile(selectedPlugin.uuid, profile.name, !profile.dontAutoSwitchWhenInstalled))}>{profile.deviceType === 7 ? 'Install & Activate' : 'Other device'}</button></div>)}</div>}
        </> : selectedPack ? <>
          <div><small>{selectedPack.id}</small><h2>{selectedPack.name}</h2><p>{selectedPack.description || 'Stream Deck-compatible icon pack.'}</p></div>
          <dl className="plugin-meta"><div><dt>Type</dt><dd>Icon Pack</dd></div><div><dt>Version</dt><dd>{selectedPack.version}</dd></div><div><dt>Icons</dt><dd>{selectedPack.itemCount}</dd></div><div><dt>Status</dt><dd>{selectedPack.active ? 'Active' : 'Inactive'}</dd></div></dl>
          <div className="profile-action-row">
            <button type="button" className="primary" disabled={busy} onClick={() => void run(selectedPack.active ? 'Deactivating pack' : 'Activating pack', () => onSetIconPackActive(selectedPack.id, !selectedPack.active))}>{selectedPack.active ? 'Deactivate Pack' : 'Activate Pack'}</button>
            <button type="button" disabled={busy} onClick={() => void run('Removing pack', () => onRemoveIconPack(selectedPack.id))}>Remove Pack</button>
          </div>
          <p className="extension-path">{selectedPack.root}</p>
        </> : selectedDownload ? <>
          <div><small>{kindLabel(selectedDownload.kind)}</small><h2>{selectedDownload.name}</h2><p>This package was detected in Downloads. Detected does not mean installed or active.</p></div>
          <dl className="plugin-meta"><div><dt>Type</dt><dd>{kindLabel(selectedDownload.kind)}</dd></div><div className="wide"><dt>Package</dt><dd>{selectedDownload.path}</dd></div></dl>
          {packageErrors[itemKey(selectedDownload)] && <p className="plugin-error" role="alert">{packageErrors[itemKey(selectedDownload)]}</p>}
          <div className="profile-action-row">
            {selectedDownload.kind === 'plugin' && <button type="button" disabled={busy} onClick={() => void installDownloaded(selectedDownload, false)}>Install</button>}
            <button type="button" className="primary" disabled={busy} onClick={() => void installDownloaded(selectedDownload, true)}>{selectedDownload.kind === 'profile' ? 'Import & Activate Profile' : 'Install & Activate'}</button>
          </div>
        </> : <p className="empty-state">Select an installed plugin, icon pack, or downloaded package.</p>}

        <div className="plugin-install-panel">
          <h3>Install package</h3>
          <label><span>Plugin, icon pack, or profile path</span><input value={manualPath} onChange={(event) => setManualPath(event.target.value)} placeholder="~/Downloads/example.streamDeckPlugin" /></label>
          <div className="profile-action-row">
            {manualIsPlugin && <button type="button" disabled={busy || !manualPath.trim()} onClick={() => void run('Installing', () => onInstallPackage(manualPath.trim(), false))}>Install Path</button>}
            <button type="button" disabled={busy || !manualPath.trim()} onClick={() => void run('Installing and activating', () => onInstallPackage(manualPath.trim(), true))}>Install Path &amp; Activate</button>
            <button type="button" disabled={busy} onClick={() => void run('Scanning Downloads', onScan)}>Scan Downloads</button>
            <button type="button" disabled={busy} onClick={() => void onOpenMarketplace()}>Marketplace</button>
          </div>
          {message && <p role="status">{message}</p>}
        </div>
      </aside>
    </div>
  </section>;
});
