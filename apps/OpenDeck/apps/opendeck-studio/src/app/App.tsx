import { useCallback, useEffect, useReducer, useRef, useState, type CSSProperties, type PointerEvent as ReactPointerEvent } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { bridge, type QualificationContext } from '../bridge';
import { createActionInstance, getActionDefinition, supportsControl } from '../model/actions';
import { createQualificationWorkspace } from '../qualify/demoWorkspace';
import {
  createDefaultWorkspace,
  allSlots,
  findSlot,
  getActivePage,
  getActiveProfile,
  resolveTouchStripPresentation,
  type Appearance,
  type AppearanceOverride,
  type ControlSelection,
  type Interaction,
  type Workspace,
} from '../model/workspace';
import { migrateLegacyKeys } from '../model/migration';
import { canRedo, canUndo, createEditorState, editorReducer, selectedSlot } from './editor-store';
import { resolveActionExecution } from './action-executor';
import { resolveHardwareExecutions } from './hardware-events';
import { TopBar } from '../components/TopBar';
import { AppSidebar, type AppSection } from '../components/AppSidebar';
import { DeviceEditor } from '../components/DeviceEditor';
import { PageNavigator } from '../components/PageNavigator';
import { ActionLibrary, type ActionLibraryMode } from '../components/ActionLibrary';
import { PropertyInspector } from '../components/PropertyInspector';
import { QualificationHarness } from '../perf/QualificationHarness';
import { AssetBrowser } from '../components/AssetBrowser';
import type { MarketplaceItem, StreamDeckInputEvent, StreamDeckStatus, TwitchDeviceCode, TwitchIdentity } from '../types';

export interface EditorAppProps {
  qualification?: QualificationContext;
}

const DEFAULT_QUALIFICATION: QualificationContext = { enabled: false, phase: 'visual' };

const INSPECTOR_MIN = 210;
const INSPECTOR_MAX = 360;
const CANONICAL_CANVAS_WIDTH = 1536;
const CANONICAL_CANVAS_HEIGHT = 1024;

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value));
}

function bindingsSupportKind(slot: ReturnType<typeof findSlot>, kind: ControlSelection['kind']): boolean {
  if (!slot) return false;
  return Object.values(slot.bindings).every((binding) => !binding || supportsControl(binding.definitionId, kind));
}

export default function EditorApp({ qualification = DEFAULT_QUALIFICATION }: EditorAppProps) {
  const qualificationMode = qualification.enabled;
  const qualificationPhase = qualification.phase;
  const [fitScale, setFitScale] = useState(() => qualificationMode ? 1 : Math.min(
    window.innerWidth / CANONICAL_CANVAS_WIDTH,
    window.innerHeight / CANONICAL_CANVAS_HEIGHT,
    1,
  ));

  useEffect(() => {
    if (qualificationMode) {
      setFitScale(1);
      return;
    }
    const updateFitScale = () => setFitScale(Math.min(
      window.innerWidth / CANONICAL_CANVAS_WIDTH,
      window.innerHeight / CANONICAL_CANVAS_HEIGHT,
      1,
    ));
    updateFitScale();
    window.addEventListener('resize', updateFitScale);
    return () => window.removeEventListener('resize', updateFitScale);
  }, [qualificationMode]);

  const [state, dispatch] = useReducer(editorReducer, undefined, () => {
    const workspace = qualificationMode ? createQualificationWorkspace() : createDefaultWorkspace();
    const initial = createEditorState(workspace);
    if (qualificationMode) {
      const dial = getActivePage(workspace).slots.dials[0];
      initial.selection = { kind: 'dial', slotId: dial.id };
    }
    return initial;
  });
  const [loading, setLoading] = useState(!qualificationMode);
  const [saveStatus, setSaveStatus] = useState<'Loading'|'Saved'|'Dirty'|'Saving'|'Error'>('Loading');
  const [assetRole, setAssetRole] = useState<{ role: 'icon'|'background'; target: 'base'|'active' } | null>(null);
  const [previewState, setPreviewState] = useState<'default'|'active'>('default');
  const [assetPreviews, setAssetPreviews] = useState<Record<string, string>>({});
  const assetPreviewCacheRef = useRef(new Map<string, string>());
  const [inspectorHeightPreview, setInspectorHeightPreview] = useState<number | null>(null);
  const initializedRef = useRef(false);
  const skipAutosaveRef = useRef(true);
  const profile = getActiveProfile(state.workspace);
  const page = getActivePage(state.workspace);
  const slot = selectedSlot(state);
  const [panel, setPanel] = useState<'none'|'connections'|'marketplace'|'settings'>('none');
  const [section, setSection] = useState<AppSection>('buttons');
  const [status, setStatus] = useState('Ready');
  const [actionMode, setActionMode] = useState<ActionLibraryMode>('keys');
  const [selectedInteraction, setSelectedInteraction] = useState<Interaction>('press');
  const [obsHost, setObsHost] = useState('127.0.0.1');
  const [obsPort, setObsPort] = useState(4455);
  const [obsPassword, setObsPassword] = useState('');
  const [obsStatus, setObsStatus] = useState('Not connected');
  const [scenes, setScenes] = useState<string[]>([]);
  const [twitchIdentity, setTwitchIdentity] = useState<TwitchIdentity | null>(null);
  const [twitchCode, setTwitchCode] = useState<TwitchDeviceCode | null>(null);
  const [twitchAuthMessage, setTwitchAuthMessage] = useState<string | null>(null);
  const twitchAttemptRef = useRef(0);
  const [marketItems, setMarketItems] = useState<MarketplaceItem[]>([]);
  const [hardwareStatus, setHardwareStatus] = useState<StreamDeckStatus>(qualificationMode
    ? { state: 'connected', model: 'Stream Deck +', serial: 'QUALIFY-V226', message: 'Connected' }
    : { state: 'disconnected', model: 'Stream Deck +', serial: null, message: 'Checking Stream Deck +…' });
  const [activeAppId, setActiveAppId] = useState<string | null>(null);
  const activeAppIdRef = useRef<string | null>(null);
  const workspaceRef = useRef(state.workspace);
  const touchPresentation = resolveTouchStripPresentation(page, activeAppId);

  const executeBinding = useCallback(async (
    targetSlot: NonNullable<ReturnType<typeof findSlot>>,
    interaction: Interaction,
    origin: 'test' | 'hardware',
    workspace: Workspace,
  ) => {
    const binding = targetSlot.bindings[interaction];
    if (!binding) {
      if (origin === 'test') setStatus('Choose an action before testing this interaction.');
      return;
    }
    const activeProfile = getActiveProfile(workspace);
    const activePage = getActivePage(workspace);
    const execution = resolveActionExecution(binding, {
      pages: activeProfile.pages.map(({ id, name }) => ({ id, name })),
      currentPageId: activePage.id,
      profiles: workspace.profiles.map(({ id, name }) => ({ id, name })),
      currentProfileId: activeProfile.id,
    });

    if (execution.kind === 'invalid' || execution.kind === 'noop') {
      setStatus(execution.message);
      return;
    }

    try {
      switch (execution.kind) {
        case 'editor.setPage':
          dispatch({ type: 'SET_ACTIVE_PAGE', pageId: execution.pageId });
          setStatus(`Opened ${activeProfile.pages.find((candidate) => candidate.id === execution.pageId)?.name ?? 'page'}.`);
          return;
        case 'editor.setProfile': {
          const target = workspace.profiles.find((candidate) => candidate.id === execution.profileId);
          dispatch({ type: 'SET_ACTIVE_PROFILE', profileId: execution.profileId });
          setStatus(`Switched to ${target?.name ?? 'profile'}.`);
          return;
        }
        case 'obs.scene':
          await bridge.obsSetScene(execution.sceneName);
          setStatus(`OBS scene changed to ${execution.sceneName}.`);
          return;
        case 'obs.toggleMute':
          await bridge.obsToggleMute(execution.inputName);
          setStatus(`Toggled OBS mute for ${execution.inputName}.`);
          return;
        case 'obs.toggleStream':
          if (origin === 'test' && !window.confirm('Toggle OBS streaming now?')) {
            setStatus('Stream toggle cancelled.');
            return;
          }
          await bridge.obsToggleStream();
          setStatus(origin === 'hardware' ? 'OBS stream toggle sent from Stream Deck +.' : 'OBS stream toggle sent.');
          return;
        case 'obs.toggleRecord':
          if (origin === 'test' && !window.confirm('Toggle OBS recording now?')) {
            setStatus('Recording toggle cancelled.');
            return;
          }
          await bridge.obsToggleRecord();
          setStatus(origin === 'hardware' ? 'OBS recording toggle sent from Stream Deck +.' : 'OBS recording toggle sent.');
          return;
        case 'marketplace.open':
          await bridge.openExternal(execution.url);
          setStatus('Opened Elgato Marketplace.');
          return;
      }
    } catch (error) {
      setStatus(`Action failed: ${String(error)}`);
    }
  }, []);

  useEffect(() => {
    workspaceRef.current = state.workspace;
  }, [state.workspace]);

  useEffect(() => {
    activeAppIdRef.current = activeAppId;
  }, [activeAppId]);

  useEffect(() => {
    if (qualificationMode || loading || page.touchStrip.mode !== 'adaptive') {
      setActiveAppId(null);
      return;
    }
    let alive = true;
    const poll = async () => {
      try {
        const context = await bridge.activeApplicationContext();
        if (alive) setActiveAppId(context.appId);
      } catch {
        if (alive) setActiveAppId(null);
      }
    };
    void poll();
    const timer = window.setInterval(() => void poll(), 750);
    return () => { alive = false; window.clearInterval(timer); };
  }, [qualificationMode, loading, page.id, page.touchStrip.mode]);

  useEffect(() => {
    if (qualificationMode) {
      initializedRef.current = true;
      skipAutosaveRef.current = true;
      setSaveStatus('Saved');
      setStatus('Ready');
      setLoading(false);
      return;
    }
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
  }, [qualificationMode]);

  useEffect(() => {
    if (qualificationMode) return;
    let alive = true;
    void bridge.twitchStatus()
      .then((identity) => {
        if (alive) setTwitchIdentity(identity);
      })
      .catch(() => undefined);
    return () => { alive = false; };
  }, [qualificationMode]);


  useEffect(() => {
    if (qualificationMode) return;
    let alive = true;
    const unlisteners: UnlistenFn[] = [];
    void bridge.streamdeckStatus()
      .then((next) => { if (alive) setHardwareStatus(next); })
      .catch((error) => {
        if (alive) setHardwareStatus({ state: 'ioError', model: 'Stream Deck +', serial: null, message: String(error) });
      });
    void listen<StreamDeckStatus>('opendeck://hardware-status', (event) => {
      if (alive) setHardwareStatus(event.payload);
    }).then((unlisten) => {
      if (alive) unlisteners.push(unlisten); else unlisten();
    }).catch(() => undefined);
    void listen<StreamDeckInputEvent>('opendeck://hardware-input', (event) => {
      if (!alive) return;
      const workspace = workspaceRef.current;
      const activePage = getActivePage(workspace);
      const presentation = resolveTouchStripPresentation(activePage, activeAppIdRef.current);
      for (const target of resolveHardwareExecutions(event.payload, activePage, presentation)) {
        const targetSlot = findSlot(activePage, target.selection);
        if (targetSlot) void executeBinding(targetSlot, target.interaction, 'hardware', workspace);
      }
    }).then((unlisten) => {
      if (alive) unlisteners.push(unlisten); else unlisten();
    }).catch(() => undefined);
    return () => {
      alive = false;
      for (const unlisten of unlisteners) unlisten();
    };
  }, [executeBinding, qualificationMode]);

  useEffect(() => {
    if (qualificationMode || !initializedRef.current || loading) return;
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
  }, [state.workspace, loading, qualificationMode]);

  useEffect(() => {
    if (qualificationMode || !initializedRef.current || loading) return;
    const timer = window.setTimeout(() => {
      void bridge.streamdeckSyncWorkspace(state.workspace, activeAppId).catch((error) => {
        setHardwareStatus((current) => ({ ...current, state: 'ioError', message: `Hardware sync failed: ${String(error)}` }));
      });
    }, 120);
    return () => window.clearTimeout(timer);
  }, [state.workspace, activeAppId, loading, qualificationMode]);

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
    setSelectedInteraction(state.selection?.kind === 'touch' ? 'touch' : 'press');
  }, [state.selection?.slotId, state.selection?.kind]);

  const preferences = state.workspace.preferences;
  const inspectorHeight = preferences.inspectorCollapsed ? 34 : (inspectorHeightPreview ?? clamp(preferences.inspectorHeight, INSPECTOR_MIN, INSPECTOR_MAX));
  const editorStyle = { '--inspector-height': `${inspectorHeight}px` } as CSSProperties;

  function selectControl(selection: ControlSelection) {
    setActionMode(selection.kind === 'key' ? 'keys' : 'dials');
    dispatch({ type: 'SELECT_CONTROL', selection });
  }

  function selectAdjacentControl(offset: -1 | 1) {
    if (!state.selection) return;
    const list = state.selection.kind === 'key' ? page.slots.keys : state.selection.kind === 'dial' ? page.slots.dials : (touchPresentation === 'unified' ? [page.touchStrip.unifiedSlot] : page.slots.touch_regions);
    const index = list.findIndex((candidate) => candidate.id === state.selection?.slotId);
    if (index < 0 || list.length === 0) return;
    const next = list[(index + offset + list.length) % list.length];
    selectControl({ kind: state.selection.kind, slotId: next.id });
  }

  function assignAction(id: string, destination = state.selection, interactionOverride?: Interaction) {
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
    const interaction: Interaction = interactionOverride
      ?? (destinationSlot.kind === 'touch' ? 'touch' : definition.defaultInteraction);
    dispatch({
      type: 'ASSIGN_ACTION_TO_CONTROL',
      selection: destination,
      interaction,
      action: createActionInstance(id),
      title: definition.label,
    });
    selectControl(destination);
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
    selectControl(destination);
    setStatus(`${copy ? 'Copied' : 'Moved'} ${source.kind} customization.`);
  }


  async function testAction(interaction: Interaction) {
    if (!slot) {
      setStatus('Select a key, dial, or touch region first.');
      return;
    }
    await executeBinding(slot, interaction, 'test', state.workspace);
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

  function cancelTwitchSignIn() {
    twitchAttemptRef.current += 1;
    setTwitchCode(null);
    setTwitchAuthMessage(null);
  }

  async function beginTwitch() {
    const attempt = twitchAttemptRef.current + 1;
    twitchAttemptRef.current = attempt;
    setTwitchCode(null);
    setTwitchAuthMessage('Starting Twitch sign-in…');
    try {
      const code = await bridge.twitchBeginAuth();
      if (twitchAttemptRef.current !== attempt) return;
      setTwitchCode(code);
      setTwitchAuthMessage('Waiting for Twitch…');
      void bridge.openExternal(code.verification_uri).catch((error) => {
        setStatus(`Could not open Twitch activation page: ${String(error)}`);
      });

      const expiresAt = Date.now() + code.expires_in * 1000;
      while (twitchAttemptRef.current === attempt) {
        const remainingMs = expiresAt - Date.now();
        if (remainingMs <= 0) break;
        await new Promise<void>((resolve) => {
          window.setTimeout(resolve, Math.min(code.interval * 1000, remainingMs));
        });
        if (twitchAttemptRef.current !== attempt) return;
        const identity = await bridge.twitchPollAuth(code.device_code);
        if (twitchAttemptRef.current !== attempt) return;
        if (identity) {
          setTwitchIdentity(identity);
          setTwitchCode(null);
          setTwitchAuthMessage(null);
          setStatus(`Signed in to Twitch as ${identity.login}.`);
          return;
        }
      }

      if (twitchAttemptRef.current === attempt) {
        setTwitchCode(null);
        setTwitchAuthMessage('Twitch sign-in expired. Start again to get a new code.');
      }
    } catch (error) {
      if (twitchAttemptRef.current !== attempt) return;
      setTwitchCode(null);
      setTwitchAuthMessage(`Twitch sign-in failed: ${String(error)}`);
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

  const canvasStyle = { '--opendeck-fit-scale': qualificationMode ? 1 : fitScale } as CSSProperties;

  return <div className="opendeck-viewport">
    <div className="canonical-canvas" style={canvasStyle}>
      <div className="app-shell v226-shell">
    {qualificationMode && <><span className="qualification-capture-token qualification-capture-token-nw" aria-hidden="true"/><span className="qualification-capture-token qualification-capture-token-se" aria-hidden="true"/></>}
    <TopBar
      workspace={state.workspace}
      hardwareStatus={hardwareStatus}
      saveStatus={saveStatus}
      canUndo={canUndo(state)}
      canRedo={canRedo(state)}
      onUndo={() => dispatch({ type: 'UNDO' })}
      onRedo={() => dispatch({ type: 'REDO' })}
      onConnections={() => setPanel('connections')}
      onMarketplace={() => { setPanel('marketplace'); void scanMarketplace(); }}
      onSettings={() => { setSection('settings'); setPanel('settings'); }}
      onSelectProfile={(profileId) => dispatch({ type: 'SET_ACTIVE_PROFILE', profileId })}
      onCreateProfile={() => dispatch({ type: 'CREATE_PROFILE' })}
      onRenameProfile={(profileId, name) => dispatch({ type: 'RENAME_PROFILE', profileId, name })}
      onDuplicateProfile={(profileId) => dispatch({ type: 'DUPLICATE_PROFILE', profileId })}
      onDeleteProfile={(profileId) => dispatch({ type: 'DELETE_PROFILE', profileId })}
    />
    <div className="app-body workspace-grid">
      <AppSidebar section={section} onSectionChange={(next) => {
        setSection(next);
        if (next === 'buttons') setActionMode('keys');
        if (next === 'dials' || next === 'touch') setActionMode('dials');
        if (next === 'plugins') { setPanel('marketplace'); void scanMarketplace(); }
        if (next === 'settings') setPanel('settings');
      }} />
      <main className="main-editor" data-layout-region="main">
        <section className="editor-column" style={editorStyle}>
          <DeviceEditor
            page={page}
            selection={state.selection}
            assetPreviews={assetPreviews}
            previewState={previewState}
            touchPresentation={touchPresentation}
            onSelect={selectControl}
            onDropControl={dropControl}
            onDropAction={(actionId, destination) => assignAction(actionId, destination)}
          />
          {profile.pages.length > 1 && <PageNavigator
            profile={profile}
            onSelect={(pageId) => dispatch({ type: 'SET_ACTIVE_PAGE', pageId })}
            onAdd={() => dispatch({ type: 'ADD_PAGE' })}
            onDuplicate={() => dispatch({ type: 'DUPLICATE_PAGE' })}
            onRename={(pageId, name) => dispatch({ type: 'RENAME_PAGE', pageId, name })}
            onDelete={() => dispatch({ type: 'DELETE_PAGE' })}
          />}
          <div className={`inspector-resize-handle${preferences.inspectorCollapsed ? ' disabled' : ''}`} role="separator" aria-orientation="horizontal" aria-label="Resize configuration" onPointerDown={beginInspectorResize} />
          <PropertyInspector
            slot={slot}
            pages={profile.pages}
            profiles={state.workspace.profiles}
            collapsed={preferences.inspectorCollapsed}
            onToggleCollapsed={() => dispatch({ type: 'UPDATE_PREFERENCES', patch: { inspectorCollapsed: !preferences.inspectorCollapsed } })}
            onConfig={(interaction, patch) => dispatch({ type: 'UPDATE_ACTION_CONFIG', interaction, patch })}
            onClear={(interaction) => dispatch({ type: 'REMOVE_ACTION', interaction })}
            interaction={selectedInteraction}
            onInteraction={setSelectedInteraction}
            onTest={(interaction) => void testAction(interaction)}
            onAppearance={(patch: Partial<Appearance>) => dispatch({ type: 'UPDATE_APPEARANCE', patch })}
            onState={(patch: AppearanceOverride) => dispatch({ type: 'UPDATE_STATE', stateName: 'active', patch })}
            onResetState={() => dispatch({ type: 'RESET_STATE', stateName: 'active' })}
            onCopyState={() => dispatch({ type: 'UPDATE_STATE', stateName: 'active', patch: { ...slot?.appearance } })}
            previewState={previewState}
            onPreviewState={setPreviewState}
            onOpenAssets={(role, target) => setAssetRole({ role, target })}
            onSelectPrevious={() => selectAdjacentControl(-1)}
            onSelectNext={() => selectAdjacentControl(1)}
            touchStrip={page.touchStrip}
            onTouchMode={(mode) => dispatch({ type: 'SET_TOUCH_STRIP_MODE', mode })}
            onTouchFallback={(presentation) => dispatch({ type: 'SET_TOUCH_STRIP_FALLBACK', presentation })}
            onTouchRules={(rules) => dispatch({ type: 'SET_TOUCH_STRIP_RULES', rules })}
          />
        </section>
      </main>
      <ActionLibrary
        mode={actionMode}
        collapsed={preferences.actionPanelCollapsed}
        onModeChange={setActionMode}
        onToggleCollapsed={() => dispatch({ type: 'UPDATE_PREFERENCES', patch: { actionPanelCollapsed: !preferences.actionPanelCollapsed } })}
        onChoose={(id) => assignAction(id, state.selection, selectedInteraction)}
        qualificationCatalogSize={qualificationMode && qualificationPhase === 'performance' ? 5000 : 0}
      />
    </div>
    <QualificationHarness enabled={qualificationMode} phase={qualificationPhase} workspace={state.workspace} startedAtMs={qualification.startedAtMs} tauriSetupMs={qualification.tauriSetupMs} />
    <div className="editor-toast" role="status" aria-live="polite">{status !== 'Ready' ? status : ''}</div>

    {assetRole && <div className="overlay asset-overlay" onMouseDown={() => setAssetRole(null)}><section className="modal asset-modal" onMouseDown={(e) => e.stopPropagation()}><AssetBrowser assets={state.workspace.assets} role={assetRole.role} loadPreviews={loadAssetPreviews} onPick={(assetId, role) => { const patch = role === 'icon' ? { iconAssetId: assetId } : { backgroundAssetId: assetId }; dispatch(assetRole.target === 'active' ? { type: 'UPDATE_STATE', stateName: 'active', patch } : { type: 'UPDATE_APPEARANCE', patch }); setAssetRole(null); }} onImport={importAsset} onClose={() => setAssetRole(null)} /></section></div>}

    {panel !== 'none' && <div className="overlay" onMouseDown={() => setPanel('none')}><section className="modal compact-modal" onMouseDown={(e) => e.stopPropagation()}><button className="close" aria-label="Close" onClick={() => setPanel('none')}>×</button>{panel === 'connections' ? <><h2>Connections</h2><div className="connection-card"><h3>OBS Studio</h3><div className="row"><input value={obsHost} onChange={(e) => setObsHost(e.target.value)} aria-label="OBS host"/><input type="number" value={obsPort} onChange={(e) => setObsPort(Number(e.target.value))} aria-label="OBS port"/></div><input type="password" value={obsPassword} onChange={(e) => setObsPassword(e.target.value)} aria-label="OBS password" placeholder="WebSocket password"/><button onClick={() => void connectObs()}>Connect</button><p>{obsStatus}</p>{scenes.length > 0 && <small>{scenes.length} scenes available</small>}</div><div className="connection-card"><h3>Twitch</h3>{twitchIdentity ? <p>Signed in as <strong>{twitchIdentity.login}</strong></p> : <><button disabled={twitchAuthMessage === 'Starting Twitch sign-in…' || twitchAuthMessage === 'Waiting for Twitch…'} onClick={() => void beginTwitch()}>Sign in with Twitch</button>{twitchAuthMessage && <p>{twitchAuthMessage}</p>}{twitchCode && <><p>Code: <code>{twitchCode.user_code}</code></p><button className="subtle" onClick={cancelTwitchSignIn}>Cancel Twitch sign-in</button></>}</>}</div></> : panel === 'marketplace' ? <><h2>Elgato Marketplace</h2><div className="row"><button onClick={() => void bridge.openExternal('https://marketplace.elgato.com')}>Open Marketplace</button><button onClick={() => void scanMarketplace()}>Scan Downloads</button></div><div className="market-list">{marketItems.length ? marketItems.map((item) => <article key={item.path}><strong>{item.name}</strong><span>{item.kind}</span><small>{item.path}</small></article>) : <p>No compatible downloaded items found.</p>}</div></> : <><h2>Settings</h2><div className="connection-card"><h3>OpenDeck+ 2.0.26</h3><p>Render-parity mode keeps DragonGlass visual geometry and responsiveness gates active.</p><div className="row"><button onClick={() => { setPanel('connections'); }}>Connections</button><button onClick={() => { setPanel('marketplace'); void scanMarketplace(); }}>Marketplace</button></div></div></>}</section></div>}
      </div>
    </div>
  </div>;
}
