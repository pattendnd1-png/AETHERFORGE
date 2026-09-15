import { useEffect, useMemo, useReducer, useRef, useState } from 'react';
import { bridge } from '../bridge';
import { createActionInstance, getActionDefinition } from '../model/actions';
import { createDefaultWorkspace, getActivePage, getActiveProfile, type Appearance, type AppearanceOverride, type Interaction } from '../model/workspace';
import { migrateLegacyKeys } from '../model/migration';
import { canRedo, canUndo, createEditorState, editorReducer, selectedSlot } from './editor-store';
import { TopBar } from '../components/TopBar';
import { DeviceEditor } from '../components/DeviceEditor';
import { PageNavigator } from '../components/PageNavigator';
import { ActionLibrary } from '../components/ActionLibrary';
import { PropertyInspector } from '../components/PropertyInspector';
import { AssetBrowser } from '../components/AssetBrowser';
import type { MarketplaceItem, TwitchDeviceCode, TwitchIdentity } from '../types';

export default function EditorApp() {
  const [state, dispatch] = useReducer(editorReducer, createEditorState(createDefaultWorkspace()));
  const [loading, setLoading] = useState(true);
  const [saveStatus, setSaveStatus] = useState<'Loading'|'Saved'|'Dirty'|'Saving'|'Error'>('Loading');
  const [assetRole, setAssetRole] = useState<'icon'|'background'|null>(null);
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
          } catch { /* legacy storage is optional migration input only */ }
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
        .catch((error) => { setSaveStatus('Error'); setStatus(`Save failed: ${String(error)}`); });
    }, 350);
    return () => window.clearTimeout(timer);
  }, [state.workspace, loading]);

  const selectedKind = slot?.kind;
  const footerLabel = useMemo(() => `${profile.name} · ${page.name}`, [profile.name, page.name]);

  function assignAction(id: string) {
    if (!slot) { setStatus('Select a key, dial, or touch region first.'); return; }
    const definition = getActionDefinition(id);
    const interaction: Interaction = slot.kind === 'touch' ? 'touch' : definition.defaultInteraction;
    dispatch({ type: 'ASSIGN_ACTION', interaction, action: createActionInstance(id) });
    dispatch({ type: 'UPDATE_APPEARANCE', patch: { title: definition.label } });
    setStatus(`Assigned ${definition.label} to ${slot.kind} ${slot.position + 1}`);
  }

  async function connectObs() {
    setObsStatus('Connecting…');
    try { await bridge.obsSaveConfig(obsHost, obsPort, obsPassword); setObsStatus(await bridge.obsStatus()); setScenes(await bridge.obsScenes()); }
    catch (error) { setObsStatus(`Error: ${String(error)}`); }
  }

  async function beginTwitch() {
    try { const code = await bridge.twitchBeginAuth(); setTwitchCode(code); await bridge.openExternal(code.verification_uri); }
    catch (error) { setStatus(String(error)); }
  }

  async function scanMarketplace() {
    try { setMarketItems(await bridge.scanMarketplace()); } catch (error) { setStatus(String(error)); }
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
    <TopBar workspace={state.workspace} profile={profile} canUndo={canUndo(state)} canRedo={canRedo(state)} onUndo={() => dispatch({ type:'UNDO' })} onRedo={() => dispatch({ type:'REDO' })} onConnections={() => setPanel('connections')} onMarketplace={() => { setPanel('marketplace'); void scanMarketplace(); }} onSelectProfile={(profileId) => dispatch({ type:'SET_ACTIVE_PROFILE', profileId })} onCreateProfile={() => dispatch({ type:'CREATE_PROFILE' })} onRenameProfile={(profileId, name) => dispatch({ type:'RENAME_PROFILE', profileId, name })} onDuplicateProfile={(profileId) => dispatch({ type:'DUPLICATE_PROFILE', profileId })} onDeleteProfile={(profileId) => dispatch({ type:'DELETE_PROFILE', profileId })} />
    <main className="workspace-grid">
      <section className="editor-column">
        <DeviceEditor page={page} selection={state.selection} onSelect={(selection) => dispatch({ type:'SELECT_CONTROL', selection })} />
        <PageNavigator profile={profile} onSelect={(pageId) => dispatch({ type:'SET_ACTIVE_PAGE', pageId })} onAdd={() => dispatch({ type:'ADD_PAGE' })} onDuplicate={() => dispatch({ type:'DUPLICATE_PAGE' })} onDelete={() => dispatch({ type:'DELETE_PAGE' })} />
        <PropertyInspector slot={slot} onConfig={(interaction, patch) => dispatch({ type:'UPDATE_ACTION_CONFIG', interaction, patch })} onClear={(interaction) => dispatch({ type:'REMOVE_ACTION', interaction })} onAppearance={(patch: Partial<Appearance>) => dispatch({ type:'UPDATE_APPEARANCE', patch })} onState={(patch: AppearanceOverride) => dispatch({ type:'UPDATE_STATE', stateName:'active', patch })} onResetState={() => dispatch({ type:'RESET_STATE', stateName:'active' })} onCopyState={() => dispatch({ type:'UPDATE_STATE', stateName:'active', patch: { ...slot?.appearance } })} onOpenAssets={setAssetRole} />
      </section>
      <ActionLibrary selectedKind={selectedKind} onChoose={assignAction} />
    </main>
    <footer className="statusbar"><span>{status}</span><span className={`save-state ${saveStatus.toLowerCase()}`}>{saveStatus}</span><span>{footerLabel} · OpenDeck 2.0.1</span></footer>

    {assetRole && <div className="overlay asset-overlay" onMouseDown={() => setAssetRole(null)}><section className="modal asset-modal" onMouseDown={(e) => e.stopPropagation()}><AssetBrowser assets={state.workspace.assets} role={assetRole} onPick={(assetId, role) => { dispatch({ type:'UPDATE_APPEARANCE', patch: role === 'icon' ? { iconAssetId: assetId } : { backgroundAssetId: assetId } }); setAssetRole(null); }} onImport={importAsset} onClose={() => setAssetRole(null)} /></section></div>}

    {panel !== 'none' && <div className="overlay" onMouseDown={() => setPanel('none')}><section className="modal compact-modal" onMouseDown={(e) => e.stopPropagation()}><button className="close" aria-label="Close" onClick={() => setPanel('none')}>×</button>{panel === 'connections' ? <><h2>Connections</h2><div className="connection-card"><h3>OBS Studio</h3><div className="row"><input value={obsHost} onChange={(e) => setObsHost(e.target.value)} aria-label="OBS host"/><input type="number" value={obsPort} onChange={(e) => setObsPort(Number(e.target.value))} aria-label="OBS port"/></div><input type="password" value={obsPassword} onChange={(e) => setObsPassword(e.target.value)} aria-label="OBS password" placeholder="WebSocket password"/><button onClick={() => void connectObs()}>Connect</button><p>{obsStatus}</p></div><div className="connection-card"><h3>Twitch</h3>{twitchIdentity ? <p>Signed in as <strong>{twitchIdentity.login}</strong></p> : <><button onClick={() => void beginTwitch()}>Sign in with Twitch</button>{twitchCode && <p>Code: <code>{twitchCode.user_code}</code></p>}</>}</div></> : <><h2>Elgato Marketplace</h2><div className="row"><button onClick={() => void bridge.openExternal('https://marketplace.elgato.com')}>Open Marketplace</button><button onClick={() => void scanMarketplace()}>Scan Downloads</button></div><div className="market-list">{marketItems.length ? marketItems.map((item) => <article key={item.path}><strong>{item.name}</strong><span>{item.kind}</span><small>{item.path}</small></article>) : <p>No compatible downloaded items found.</p>}</div></>}</section></div>}
  </div>;
}
