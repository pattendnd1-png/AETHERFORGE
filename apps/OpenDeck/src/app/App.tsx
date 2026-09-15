import { useCallback, useEffect, useMemo, useReducer, useRef, useState, type CSSProperties, type PointerEvent as ReactPointerEvent } from 'react';
import { bridge } from '../bridge';
import { createActionInstance, getActionDefinition, supportsControl } from '../model/actions';
import {
  createDefaultWorkspace,
  allSlots,
  findSlot,
  getActivePage,
  getActiveProfile,
  type Appearance,
  type AppearanceOverride,
  type ControlSelection,
  type Interaction,
} from '../model/workspace';
import { migrateLegacyKeys } from '../model/migration';
import { canRedo, canUndo, createEditorState, editorReducer, selectedSlot } from './editor-store';
import { TopBar } from '../components/TopBar';
import { DeviceEditor } from '../components/DeviceEditor';
import { PageNavigator } from '../components/PageNavigator';
import { ActionLibrary } from '../components/ActionLibrary';
import { PropertyInspector } from '../components/PropertyInspector';
import { AssetBrowser } from '../components/AssetBrowser';
import type { MarketplaceItem, TwitchDeviceCode, TwitchIdentity } from '../types';

const ACTION_PANEL_MIN = 240;
const ACTION_PANEL_MAX = 360;
const INSPECTOR_MIN = 170;
const INSPECTOR_MAX = 320;

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function bindingsSupportKind(slot: ReturnType<typeof findSlot>, kind: ControlSelection['kind']): boolean {
  if (!slot) return false;
  return Object.values(slot.bindings).every((binding) => !binding || supportsControl(binding.definitionId, kind));
}

export default function EditorApp() {
  const [state, dispatch] = useReducer(editorReducer, createEditorState(createDefaultWorkspace()));
  const [loading, setLoading] = useState(true);
  const [saveStatus, setSaveStatus] = useState<'Loading'|'Saved'|'Dirty'|'Saving'|'Error'>('Loading');
  const [assetRole, setAssetRole] = useState<{ role: 'icon'|'background'; target: 'base'|'active' } | null>(null);
  const [previewState, setPreviewState] = useState<'default'|'active'>('default');
  const [assetPreviews, setAssetPreviews] = useState<Record<string, string>>({});
  const assetPreviewCacheRef = useRef(new Map<string, string>());
  const [actionWidthPreview, setActionWidthPreview] = useState<number | null>(null);
  const [inspectorHeightPreview, setInspectorHeightPreview] = useState<number | null>(null);
  const initializedRef = useRef(false);
  const skipAutosaveRef = useRef(true);
  const profile = getActiveProfile(state.workspace);
  const page = getActivePage(state.workspace);
  const slot = selectedSlot(state);
  const [panel, setPanel] = useState<'none'|'connections'|'marketplace'>('none');
  const [status, setStatus] = useState('Ready');
  const [obsHost, setObsHost] = useState('127.0.0.1');
  const [obsPort, setObsPort] = useState(4455);
  const [obsPassword, setObsPassword] = useState('');
  const [obsStatus, setObsStatus] = useState('Not connected');
  const [scenes, setScenes] = useState<string[]>([]);
  const [twitchIdentity, setTwitchIdentity] = useState<TwitchIdentity | null>(null);
  const [twitchCode, setTwitchCode] = useState<TwitchDeviceCode | null>(null);
  const [marketItems, setMarketItems] = useState<MarketplaceItem[]>([]);

  useEffect(() => {
    let alive = true;
    void (async () => {
      try {
        const loaded = await bridge.editorLoadWorkspace();
        let workspace = loaded.workspace;
        let migrated = false;
        if (loaded.source === 'default') {
          try {
            const raw = window.localStorage.getItem('opendeck-v2.keys');
            if (raw) {
              const result = migrateLegacyKeys(JSON.parse(raw), workspace);
              workspace = result.workspace;
              migrated = result.migrated;
            }
          } catch {
            // Legacy storage is optional migration input only.
          }
        }
        if (migrated) {
          await bridge.editorSaveWorkspace(workspace);
          window.localStorage.removeItem('opendeck-v2.keys');
        }
        const assets = await bridge.editorListAssets().catch(() => workspace.assets);
        workspace = { ...workspace, assets };
        if (!alive) return;
        dispatch({ type: 'REPLACE_WORKSPACE', workspace });
        setStatus(loaded.warning ?? 'Ready');
        setSaveStatus('Saved');
        skipAutosaveRef.current = true;
        initializedRef.current = true;
      } catch (error) {
        if (!alive) return;
        setStatus(`Editor storage warning: ${String(error)}`);
        setSaveStatus('Error');
        initializedRef.current = true;
      } finally {
        if (alive) setLoading(false);
      }
    })();
    return () => { alive = false; };
  }, []);

  useEffect(() => {
    if (!initializedRef.current || loading) return;
    if (skipAutosaveRef.current) {
      skipAutosaveRef.current = false;
      return;
    }
    setSaveStatus('Dirty');
    const timer = window.setTimeout(() => {
      setSaveStatus('Saving');
      void bridge.editorSaveWorkspace(state.workspace)
        .then(() => setSaveStatus('Saved'))
        .catch((error) => {
          setSaveStatus('Error');
          setStatus(`Save failed: ${String(error)}`);
        });
    }, 350);
    return () => window.clearTimeout(timer);
  }, [state.workspace, loading]);

  const loadAssetPreviews = useCallback(async (assetIds: string[]) => {
    const ids = [...new Set(assetIds)].filter(Boolean);
    const result: Record<string, string> = {};
    const missing: string[] = [];
    for (const id of ids) {
      const cached = assetPreviewCacheRef.current.get(id);
      if (cached === undefined) missing.push(id);
      else result[id] = cached;
    }
    if (missing.length > 0) {
      const loaded = await bridge.editorAssetDataUrls(missing);
      const additions: Record<string, string> = {};
      for (const id of missing) {
        const preview = loaded[id] ?? '';
        assetPreviewCacheRef.current.set(id, preview);
        additions[id] = preview;
        result[id] = preview;
      }
      setAssetPreviews((current) => ({ ...current, ...additions }));
    }
    return result;
  }, []);

  useEffect(() => {
    const ids = new Set<string>();
    for (const control of allSlots(page)) {
      if (control.appearance.iconAssetId) ids.add(control.appearance.iconAssetId);
      if (control.appearance.backgroundAssetId) ids.add(control.appearance.backgroundAssetId);
      for (const stateAppearance of Object.values(control.states)) {
        if (stateAppearance.iconAssetId) ids.add(stateAppearance.iconAssetId);
        if (stateAppearance.backgroundAssetId) ids.add(stateAppearance.backgroundAssetId);
      }
    }
    if (ids.size > 0) {
      void loadAssetPreviews([...ids]).catch(() => {
        // Preview failures must not block the editor or persistence.
      });
    }
  }, [page, loadAssetPreviews]);

  useEffect(() => {
    setPreviewState('default');
  }, [state.selection?.slotId, state.selection?.kind]);

  const selectedKind = slot?.kind;
  const footerLabel = useMemo(() => `${profile.name} · ${page.name}`, [profile.name, page.name]);
  const preferences = state.workspace.preferences;
  const actionWidth = preferences.actionPanelCollapsed ? 34 : (actionWidthPreview ?? preferences.actionPanelWidth);
  const inspectorHeight = preferences.inspectorCollapsed ? 34 : (inspectorHeightPreview ?? preferences.inspectorHeight);
  const workspaceStyle = { '--action-panel-width': `${actionWidth}px` } as CSSProperties;
  const editorStyle = { '--inspector-height': `${inspectorHeight}px` } as CSSProperties;

  function assignAction(id: string, destination = state.selection) {
    if (!destination) {
      setStatus('Select a key, dial, or touch region first.');
      return;
    }
    const destinationSlot = findSlot(page, destination);
    if (!destinationSlot) {
      setStatus('The selected control is no longer available.');
      return;
    }
    const definition = getActionDefinition(id);
    if (!supportsControl(id, destinationSlot.kind)) {
      setStatus(`${definition.label} is not supported on ${destinationSlot.kind} controls.`);
      return;
    }
    const interaction: Interaction = destinationSlot.kind === 'touch' ? 'touch' : definition.defaultInteraction;
    dispatch({
      type: 'ASSIGN_ACTION_TO_CONTROL',
      selection: destination,
      interaction,
      action: createActionInstance(id),
      title: definition.label,
    });
    dispatch({ type: 'SELECT_CONTROL', selection: destination });
    setStatus(`Assigned ${definition.label} to ${destinationSlot.kind} ${destinationSlot.position + 1}`);
  }

  function dropControl(source: ControlSelection, destination: ControlSelection, copy: boolean) {
    const sourceSlot = findSlot(page, source);
    const destinationSlot = findSlot(page, destination);
    if (!sourceSlot || !destinationSlot) return;
    if (!bindingsSupportKind(sourceSlot, destination.kind)) {
      setStatus(`That control contains an action that is not supported on ${destination.kind}.`);
      return;
    }
    if (!copy && !bindingsSupportKind(destinationSlot, source.kind)) {
      setStatus(`The destination contains an action that is not supported on ${source.kind}.`);
      return;
    }
    if (copy) {
      dispatch({ type: 'COPY_CONTROL', source, destination });
    } else {
      dispatch({ type: 'MOVE_CONTROL', source, destination });
    }
    dispatch({ type: 'SELECT_CONTROL', selection: destination });
    setStatus(`${copy ? 'Copied' : 'Moved'} ${source.kind} customization.`);
  }

  function beginActionResize(event: ReactPointerEvent<HTMLDivElement>) {
    if (preferences.actionPanelCollapsed) return;
    event.preventDefault();
    const startX = event.clientX;
    const startWidth = preferences.actionPanelWidth;
    let latest = startWidth;
    const move = (pointerEvent: PointerEvent) => {
      latest = clamp(startWidth + startX - pointerEvent.clientX, ACTION_PANEL_MIN, ACTION_PANEL_MAX);
      setActionWidthPreview(latest);
    };
    const up = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      setActionWidthPreview(null);
      dispatch({ type: 'UPDATE_PREFERENCES', patch: { actionPanelWidth: latest } });
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up, { once: true });
  }

  function beginInspectorResize(event: ReactPointerEvent<HTMLDivElement>) {
    if (preferences.inspectorCollapsed) return;
    event.preventDefault();
    const startY = event.clientY;
    const startHeight = preferences.inspectorHeight;
    let latest = startHeight;
    const move = (pointerEvent: PointerEvent) => {
      latest = clamp(startHeight + startY - pointerEvent.clientY, INSPECTOR_MIN, INSPECTOR_MAX);
      setInspectorHeightPreview(latest);
    };
    const up = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      setInspectorHeightPreview(null);
      dispatch({ type: 'UPDATE_PREFERENCES', patch: { inspectorHeight: latest } });
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up, { once: true });
  }

  async function connectObs() {
    setObsStatus('Connecting…');
    try {
      await bridge.obsSaveConfig(obsHost, obsPort, obsPassword);
      setObsStatus(await bridge.obsStatus());
      setScenes(await bridge.obsScenes());
    } catch (error) {
      setObsStatus(`Error: ${String(error)}`);
    }
  }

  async function beginTwitch() {
    try {
      const code = await bridge.twitchBeginAuth();
      setTwitchCode(code);
      await bridge.openExternal(code.verification_uri);
    } catch (error) {
      setStatus(String(error));
    }
  }

  async function scanMarketplace() {
    try {
      setMarketItems(await bridge.scanMarketplace());
    } catch (error) {
      setStatus(String(error));
    }
  }

  async function importAsset(path: string) {
    try {
      const asset = await bridge.editorImportAsset(path);
      dispatch({ type: 'UPSERT_ASSET', asset });
      setStatus(`Imported ${asset.name}`);
    } catch (error) {
      setStatus(`Asset import failed: ${String(error)}`);
    }
  }

  if (loading) return <div className="loading-screen">Loading OpenDeck editor…</div>;

  return <div className="app-shell v201-shell">
    <TopBar
      workspace={state.workspace}
      profile={profile}
      canUndo={canUndo(state)}
      canRedo={canRedo(state)}
      onUndo={() => dispatch({ type: 'UNDO' })}
      onRedo={() => dispatch({ type: 'REDO' })}
      onConnections={() => setPanel('connections')}
      onMarketplace={() => { setPanel('marketplace'); void scanMarketplace(); }}
      onSelectProfile={(profileId) => dispatch({ type: 'SET_ACTIVE_PROFILE', profileId })}
      onCreateProfile={() => dispatch({ type: 'CREATE_PROFILE' })}
      onRenameProfile={(profileId, name) => dispatch({ type: 'RENAME_PROFILE', profileId, name })}
      onDuplicateProfile={(profileId) => dispatch({ type: 'DUPLICATE_PROFILE', profileId })}
      onDeleteProfile={(profileId) => dispatch({ type: 'DELETE_PROFILE', profileId })}
    />
    <main className="workspace-grid" style={workspaceStyle}>
      <section className="editor-column" style={editorStyle}>
        <DeviceEditor
          page={page}
          selection={state.selection}
          assetPreviews={assetPreviews}
          previewState={previewState}
          onSelect={(selection) => dispatch({ type: 'SELECT_CONTROL', selection })}
          onDropControl={dropControl}
          onDropAction={(actionId, destination) => assignAction(actionId, destination)}
        />
        <PageNavigator
          profile={profile}
          onSelect={(pageId) => dispatch({ type: 'SET_ACTIVE_PAGE', pageId })}
          onAdd={() => dispatch({ type: 'ADD_PAGE' })}
          onDuplicate={() => dispatch({ type: 'DUPLICATE_PAGE' })}
          onRename={(pageId, name) => dispatch({ type: 'RENAME_PAGE', pageId, name })}
          onDelete={() => dispatch({ type: 'DELETE_PAGE' })}
        />
        <div className={`inspector-resize-handle${preferences.inspectorCollapsed ? ' disabled' : ''}`} role="separator" aria-orientation="horizontal" aria-label="Resize property inspector" onPointerDown={beginInspectorResize} />
        <PropertyInspector
          slot={slot}
          pages={profile.pages}
          profiles={state.workspace.profiles}
          collapsed={preferences.inspectorCollapsed}
          onToggleCollapsed={() => dispatch({ type: 'UPDATE_PREFERENCES', patch: { inspectorCollapsed: !preferences.inspectorCollapsed } })}
          onConfig={(interaction, patch) => dispatch({ type: 'UPDATE_ACTION_CONFIG', interaction, patch })}
          onClear={(interaction) => dispatch({ type: 'REMOVE_ACTION', interaction })}
          onAppearance={(patch: Partial<Appearance>) => dispatch({ type: 'UPDATE_APPEARANCE', patch })}
          onState={(patch: AppearanceOverride) => dispatch({ type: 'UPDATE_STATE', stateName: 'active', patch })}
          onResetState={() => dispatch({ type: 'RESET_STATE', stateName: 'active' })}
          onCopyState={() => dispatch({ type: 'UPDATE_STATE', stateName: 'active', patch: { ...slot?.appearance } })}
          previewState={previewState}
          onPreviewState={setPreviewState}
          onOpenAssets={(role, target) => setAssetRole({ role, target })}
        />
      </section>
      <div className={`action-resize-handle${preferences.actionPanelCollapsed ? ' disabled' : ''}`} role="separator" aria-orientation="vertical" aria-label="Resize action panel" onPointerDown={beginActionResize} />
      <ActionLibrary
        selectedKind={selectedKind}
        collapsed={preferences.actionPanelCollapsed}
        onToggleCollapsed={() => dispatch({ type: 'UPDATE_PREFERENCES', patch: { actionPanelCollapsed: !preferences.actionPanelCollapsed } })}
        onChoose={assignAction}
      />
    </main>
    <footer className="statusbar"><span>{status}</span><span className={`save-state ${saveStatus.toLowerCase()}`}>{saveStatus}</span><span>{footerLabel} · OpenDeck 2.0.1</span></footer>

    {assetRole && <div className="overlay asset-overlay" onMouseDown={() => setAssetRole(null)}><section className="modal asset-modal" onMouseDown={(e) => e.stopPropagation()}><AssetBrowser assets={state.workspace.assets} role={assetRole.role} loadPreviews={loadAssetPreviews} onPick={(assetId, role) => { const patch = role === 'icon' ? { iconAssetId: assetId } : { backgroundAssetId: assetId }; dispatch(assetRole.target === 'active' ? { type: 'UPDATE_STATE', stateName: 'active', patch } : { type: 'UPDATE_APPEARANCE', patch }); setAssetRole(null); }} onImport={importAsset} onClose={() => setAssetRole(null)} /></section></div>}

    {panel !== 'none' && <div className="overlay" onMouseDown={() => setPanel('none')}><section className="modal compact-modal" onMouseDown={(e) => e.stopPropagation()}><button className="close" aria-label="Close" onClick={() => setPanel('none')}>×</button>{panel === 'connections' ? <><h2>Connections</h2><div className="connection-card"><h3>OBS Studio</h3><div className="row"><input value={obsHost} onChange={(e) => setObsHost(e.target.value)} aria-label="OBS host"/><input type="number" value={obsPort} onChange={(e) => setObsPort(Number(e.target.value))} aria-label="OBS port"/></div><input type="password" value={obsPassword} onChange={(e) => setObsPassword(e.target.value)} aria-label="OBS password" placeholder="WebSocket password"/><button onClick={() => void connectObs()}>Connect</button><p>{obsStatus}</p>{scenes.length > 0 && <small>{scenes.length} scenes available</small>}</div><div className="connection-card"><h3>Twitch</h3>{twitchIdentity ? <p>Signed in as <strong>{twitchIdentity.login}</strong></p> : <><button onClick={() => void beginTwitch()}>Sign in with Twitch</button>{twitchCode && <p>Code: <code>{twitchCode.user_code}</code></p>}</>}</div></> : <><h2>Elgato Marketplace</h2><div className="row"><button onClick={() => void bridge.openExternal('https://marketplace.elgato.com')}>Open Marketplace</button><button onClick={() => void scanMarketplace()}>Scan Downloads</button></div><div className="market-list">{marketItems.length ? marketItems.map((item) => <article key={item.path}><strong>{item.name}</strong><span>{item.kind}</span><small>{item.path}</small></article>) : <p>No compatible downloaded items found.</p>}</div></>}</section></div>}
  </div>;
}
