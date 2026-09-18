import { useCallback, useEffect, useMemo, useReducer, useRef, useState, type CSSProperties, type PointerEvent as ReactPointerEvent } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { bridge, type QualificationContext } from '../bridge';
import { createActionInstance, getActionDefinition, supportsControl } from '../model/actions';
import { isPluginDefinitionId, parsePluginDefinitionId, replacePluginActionDefinitions } from '../model/plugin-actions';
import { actionWheelStatus } from '../model/action-wheel';
import { dialStackStatus, effectiveDialBindings } from '../model/dial-stack';
import { createQualificationWorkspace } from '../qualify/demoWorkspace';
import {
  createDefaultWorkspace,
  allSlots,
  findSlot,
  getActivePage,
  getActiveProfile,
  resolveTouchStripPresentation,
  type ActionInstance,
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
import { ProfilesWorkspace } from '../components/ProfilesWorkspace';
import { PluginManager } from '../components/PluginManager';
import { WindowResizeFrame } from '../components/WindowResizeFrame';
import { applyPluginFeedback } from '../plugins/feedback';
import type { PluginDescriptor, PluginFeedback, PluginHostStatus } from '../plugins/types';
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
  if ((slot.dialStack || slot.actionWheel) && kind !== 'dial') return false;
  const maps = [slot.bindings, ...(slot.dialStack?.entries.map((entry) => entry.bindings) ?? []), ...(slot.actionWheel?.entries.map((entry) => entry.bindings) ?? [])];
  return maps.every((bindings) => Object.values(bindings).every((binding) => !binding || supportsControl(binding.definitionId, kind)));
}

function actionTreeContainsPlugin(action: ActionInstance | undefined): boolean {
  if (!action) return false;
  if (isPluginDefinitionId(action.definitionId)) return true;
  if (action.definitionId === 'editor.multiAction' && Array.isArray(action.config.steps)) {
    return (action.config.steps as ActionInstance[]).some(actionTreeContainsPlugin);
  }
  if (action.definitionId === 'editor.keyLogic') {
    return (['press', 'doublePress', 'longPress'] as const).some((trigger) => {
      const nested = action.config[trigger];
      return nested && typeof nested === 'object' ? actionTreeContainsPlugin(nested as ActionInstance) : false;
    });
  }
  return false;
}

interface PluginLifecycleEntry {
  pluginUuid: string;
  actionUuid: string;
  context: string;
  device: string;
  controlKind: ControlSelection['kind'];
  position: number;
  isInMultiAction: boolean;
}

function activePluginLifecycleEntries(workspace: Workspace): Map<string, PluginLifecycleEntry> {
  const page = getActivePage(workspace);
  const result = new Map<string, PluginLifecycleEntry>();
  const visit = (action: ActionInstance | undefined, slot: NonNullable<ReturnType<typeof findSlot>>, isInMultiAction: boolean) => {
    if (!action) return;
    if (action.definitionId === 'editor.multiAction' && Array.isArray(action.config.steps)) {
      for (const step of action.config.steps as ActionInstance[]) visit(step, slot, true);
      return;
    }
    if (action.definitionId === 'editor.keyLogic') {
      for (const trigger of ['press', 'doublePress', 'longPress'] as const) {
        const nested = action.config[trigger];
        if (nested && typeof nested === 'object') visit(nested as ActionInstance, slot, false);
      }
      return;
    }
    const parsed = parsePluginDefinitionId(action.definitionId);
    const context = typeof action.config.context === 'string' ? action.config.context : '';
    if (!parsed || !context) return;
    result.set(context, { pluginUuid: parsed.pluginUuid, actionUuid: parsed.actionUuid, context, device: workspace.active_device_id, controlKind: slot.kind, position: slot.position, isInMultiAction });
  };
  for (const slot of allSlots(page)) {
    for (const action of Object.values(effectiveDialBindings(slot))) visit(action, slot, false);
  }
  return result;
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
  const [plugins, setPlugins] = useState<PluginDescriptor[]>([]);
  const [pluginHostStatus, setPluginHostStatus] = useState<PluginHostStatus | null>(null);
  const [pluginFeedback, setPluginFeedback] = useState<Record<string, PluginFeedback>>({});
  const externalActions = useMemo(() => replacePluginActionDefinitions(plugins), [plugins]);
  const [hardwareStatus, setHardwareStatus] = useState<StreamDeckStatus>(qualificationMode
    ? { state: 'connected', model: 'Stream Deck +', serial: 'QUALIFY-V244', message: 'Connected' }
    : { state: 'disconnected', model: 'Stream Deck +', serial: null, message: 'Checking Stream Deck +…' });
  const [activeAppId, setActiveAppId] = useState<string | null>(null);
  const activeAppIdRef = useRef<string | null>(null);
  const workspaceRef = useRef(state.workspace);
  const visiblePluginContextsRef = useRef(new Map<string, PluginLifecycleEntry>());
  const previousPluginProfileRef = useRef(new Map<string, string>());
  const keyLogicPressesRef = useRef(new Map<string, {
    holdTimer: number | null;
    pendingSingleTimer: number | null;
    holdTriggered: boolean;
    doubleCandidate: boolean;
  }>());
  const touchPresentation = resolveTouchStripPresentation(page, activeAppId);
  const appliedPluginFeedback = useMemo(() => applyPluginFeedback(state.workspace, pluginFeedback), [state.workspace, pluginFeedback]);
  const runtimeWorkspace = appliedPluginFeedback.workspace;
  const runtimePage = getActivePage(runtimeWorkspace);
  const runtimeAssetPreviews = useMemo(() => ({ ...assetPreviews, ...appliedPluginFeedback.previewDataUrls }), [assetPreviews, appliedPluginFeedback.previewDataUrls]);

  const refreshPlugins = useCallback(async () => {
    const [catalog, host] = await Promise.all([bridge.pluginList(), bridge.pluginHostStatus()]);
    setPlugins(catalog);
    setPluginHostStatus(host);
  }, []);

  const executeActionInstance = useCallback(async (
    binding: ActionInstance,
    targetSlot: NonNullable<ReturnType<typeof findSlot>>,
    interaction: Interaction,
    origin: 'test' | 'hardware',
    workspace: Workspace,
    hardwareEvent?: StreamDeckInputEvent,
    isInMultiAction = false,
  ) => {
    const run = async (current: ActionInstance, nestedMultiAction: boolean, depth: number): Promise<void> => {
      if (depth > 32) throw new Error('Multi Action depth limit reached.');
      if (current.definitionId === 'editor.multiAction') {
        const steps = Array.isArray(current.config.steps) ? current.config.steps as ActionInstance[] : [];
        for (const step of steps) await run(step, true, depth + 1);
        return;
      }
      if (current.definitionId === 'editor.keyLogic') {
        const pressAction = current.config.press;
        if (pressAction && typeof pressAction === 'object') await run(pressAction as ActionInstance, nestedMultiAction, depth + 1);
        else setStatus('Key Logic Press branch is unassigned.');
        return;
      }
      if (isPluginDefinitionId(current.definitionId)) {
        const parsed = parsePluginDefinitionId(current.definitionId);
        if (!parsed) throw new Error(`Invalid plugin action id: ${current.definitionId}`);
        const context = typeof current.config.context === 'string' && current.config.context
          ? current.config.context
          : `plugin-context-${targetSlot.id}-${interaction}`;
        await bridge.pluginDispatchAction({
          pluginUuid: parsed.pluginUuid,
          actionUuid: parsed.actionUuid,
          context,
          device: workspace.active_device_id,
          controlKind: targetSlot.kind,
          position: targetSlot.position,
          interaction,
          hardwareEvent,
          isInMultiAction: nestedMultiAction || isInMultiAction,
          userDesiredState: typeof current.config.state === 'number' ? current.config.state : null,
        });
        setStatus(`${getActionDefinition(current.definitionId).label} dispatched${nestedMultiAction || isInMultiAction ? ' in Multi Action' : ''}.`);
        return;
      }
      const activeProfile = getActiveProfile(workspace);
      const activePage = getActivePage(workspace);
      const execution = resolveActionExecution(current, {
        pages: activeProfile.pages.map(({ id, name }) => ({ id, name })),
        currentPageId: activePage.id,
        profiles: workspace.profiles.map(({ id, name }) => ({ id, name })),
        currentProfileId: activeProfile.id,
      });
      if (execution.kind === 'invalid' || execution.kind === 'noop') {
        setStatus(execution.message);
        return;
      }
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
          if (origin === 'test' && !nestedMultiAction && !window.confirm('Toggle OBS streaming now?')) {
            setStatus('Stream toggle cancelled.');
            return;
          }
          await bridge.obsToggleStream();
          setStatus(origin === 'hardware' ? 'OBS stream toggle sent from Stream Deck +.' : 'OBS stream toggle sent.');
          return;
        case 'obs.toggleRecord':
          if (origin === 'test' && !nestedMultiAction && !window.confirm('Toggle OBS recording now?')) {
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
    };
    try {
      await run(binding, isInMultiAction, 0);
    } catch (error) {
      setStatus(`Action failed: ${String(error)}`);
    }
  }, []);

  const executeBinding = useCallback(async (
    targetSlot: NonNullable<ReturnType<typeof findSlot>>,
    interaction: Interaction,
    origin: 'test' | 'hardware',
    workspace: Workspace,
    hardwareEvent?: StreamDeckInputEvent,
  ) => {
    if (targetSlot.kind === 'dial' && targetSlot.actionWheel && (interaction === 'rotateLeft' || interaction === 'pressRotateLeft' || interaction === 'rotateRight' || interaction === 'pressRotateRight')) {
      const direction = interaction === 'rotateLeft' || interaction === 'pressRotateLeft' ? -1 : 1;
      dispatch({ type: 'STEP_ACTION_WHEEL', selection: { kind: 'dial', slotId: targetSlot.id }, direction });
      const current = actionWheelStatus(targetSlot);
      if (origin === 'test' || current) setStatus(current ? `Action Wheel: moving from ${current.label}.` : 'Action Wheel selection changed.');
      return;
    }
    if (targetSlot.kind === 'dial' && targetSlot.dialStack && interaction === 'press') {
      const current = dialStackStatus(targetSlot);
      dispatch({ type: 'CYCLE_DIAL_STACK', selection: { kind: 'dial', slotId: targetSlot.id } });
      if (origin === 'test' || current) setStatus(current ? `Dial Stack: cycling from ${current.label}.` : 'Dial Stack cycled.');
      return;
    }
    const binding = effectiveDialBindings(targetSlot)[interaction];
    if (!binding) {
      if (origin === 'test') setStatus('Choose an action before testing this interaction.');
      return;
    }
    await executeActionInstance(binding, targetSlot, interaction, origin, workspace, hardwareEvent);
  }, [executeActionInstance]);

  const handleKeyLogicHardware = useCallback((
    kind: 'keyDown' | 'keyUp',
    targetSlot: NonNullable<ReturnType<typeof findSlot>>,
    binding: ActionInstance,
  ): boolean => {
    if (binding.definitionId !== 'editor.keyLogic' || targetSlot.kind !== 'key') return false;
    const key = targetSlot.id;
    const holdMs = typeof binding.config.holdMs === 'number' ? clamp(binding.config.holdMs, 250, 2000) : 500;
    const doubleMs = typeof binding.config.doubleMs === 'number' ? clamp(binding.config.doubleMs, 150, 750) : 300;
    const fire = (trigger: 'press' | 'doublePress' | 'longPress') => {
      const nested = binding.config[trigger];
      if (!nested || typeof nested !== 'object') { setStatus(`Key Logic ${trigger} branch is unassigned.`); return; }
      void executeActionInstance(nested as ActionInstance, targetSlot, 'press', 'hardware', workspaceRef.current);
    };
    const previous = keyLogicPressesRef.current.get(key) ?? { holdTimer: null, pendingSingleTimer: null, holdTriggered: false, doubleCandidate: false };
    if (kind === 'keyDown') {
      const doubleCandidate = previous.pendingSingleTimer !== null;
      if (previous.pendingSingleTimer !== null) window.clearTimeout(previous.pendingSingleTimer);
      if (previous.holdTimer !== null) window.clearTimeout(previous.holdTimer);
      const next = { ...previous, pendingSingleTimer: null, holdTriggered: false, doubleCandidate, holdTimer: null as number | null };
      next.holdTimer = window.setTimeout(() => {
        const live = keyLogicPressesRef.current.get(key);
        if (!live) return;
        live.holdTriggered = true;
        live.doubleCandidate = false;
        live.pendingSingleTimer = null;
        fire('longPress');
      }, holdMs);
      keyLogicPressesRef.current.set(key, next);
      return true;
    }
    const live = keyLogicPressesRef.current.get(key) ?? previous;
    if (live.holdTimer !== null) window.clearTimeout(live.holdTimer);
    live.holdTimer = null;
    if (live.holdTriggered) { keyLogicPressesRef.current.delete(key); return true; }
    if (live.doubleCandidate) {
      if (live.pendingSingleTimer !== null) window.clearTimeout(live.pendingSingleTimer);
      keyLogicPressesRef.current.delete(key);
      fire('doublePress');
      return true;
    }
    live.pendingSingleTimer = window.setTimeout(() => {
      keyLogicPressesRef.current.delete(key);
      fire('press');
    }, doubleMs);
    keyLogicPressesRef.current.set(key, live);
    return true;
  }, [executeActionInstance]);

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
    if (qualificationMode || loading || !activeAppId) return;
    void bridge.pluginDispatchHostEvent('applicationDidLaunch', { application: activeAppId }, undefined, activeAppId).catch(() => undefined);
    return () => { void bridge.pluginDispatchHostEvent('applicationDidTerminate', { application: activeAppId }, undefined, activeAppId).catch(() => undefined); };
  }, [activeAppId, loading, qualificationMode]);

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
    void refreshPlugins().catch((error) => { if (alive) setStatus(`Plugin host warning: ${String(error)}`); });
    void listen<PluginDescriptor[]>('opendeck://plugin-catalog', (event) => {
      if (alive) { setPlugins(event.payload); void bridge.pluginHostStatus().then((host) => alive && setPluginHostStatus(host)).catch(() => undefined); }
    }).then((unlisten) => { if (alive) unlisteners.push(unlisten); else unlisten(); }).catch(() => undefined);
    void listen<PluginFeedback>('opendeck://plugin-feedback', (event) => {
      if (!alive || !event.payload.context) return;
      setPluginFeedback((current) => ({ ...current, [event.payload.context]: { ...(current[event.payload.context] ?? {}), ...event.payload } }));
      if (event.payload.signal === 'ok') setStatus('Plugin action completed.');
      if (event.payload.signal === 'alert') setStatus('Plugin reported an alert.');
    }).then((unlisten) => { if (alive) unlisteners.push(unlisten); else unlisten(); }).catch(() => undefined);
    void listen<Record<string, unknown>>('opendeck://plugin-switch-profile', (event) => {
      if (!alive) return;
      const pluginUuid = typeof event.payload?.pluginUuid === 'string' ? event.payload.pluginUuid : '';
      if (!pluginUuid) return;
      const payload = event.payload?.payload && typeof event.payload.payload === 'object'
        ? event.payload.payload as Record<string, unknown>
        : {};
      const profileValue = payload.profile;
      const pageValue = typeof payload.page === 'number' ? Math.max(0, Math.floor(payload.page)) : null;
      const current = getActiveProfile(workspaceRef.current);
      if (typeof profileValue !== 'string') {
        const previousId = previousPluginProfileRef.current.get(pluginUuid);
        const previous = previousId ? workspaceRef.current.profiles.find((candidate) => candidate.id === previousId) : undefined;
        if (previous) dispatch({ type: 'SET_ACTIVE_PROFILE', profileId: previous.id });
        return;
      }
      const target = workspaceRef.current.profiles.find((candidate) => candidate.pluginOwnerUuid === pluginUuid && candidate.name === profileValue);
      if (!target) { setStatus(`Plugin profile ${profileValue} is not installed for ${pluginUuid}.`); return; }
      if (current.id !== target.id) previousPluginProfileRef.current.set(pluginUuid, current.id);
      dispatch({ type: 'SET_ACTIVE_PROFILE', profileId: target.id });
      if (pageValue !== null && target.pages[pageValue]) dispatch({ type: 'SET_ACTIVE_PAGE', pageId: target.pages[pageValue].id });
    }).then((unlisten) => { if (alive) unlisteners.push(unlisten); else unlisten(); }).catch(() => undefined);
    return () => { alive = false; for (const unlisten of unlisteners) unlisten(); };
  }, [qualificationMode, refreshPlugins]);

  useEffect(() => {
    if (qualificationMode || loading) return;
    const previous = visiblePluginContextsRef.current;
    const next = activePluginLifecycleEntries(state.workspace);
    for (const [context, entry] of previous) {
      if (!next.has(context)) void bridge.pluginDispatchLifecycle({ ...entry, event: 'willDisappear' }).catch(() => undefined);
    }
    for (const [context, entry] of next) {
      if (!previous.has(context)) void bridge.pluginDispatchLifecycle({ ...entry, event: 'willAppear' }).catch(() => undefined);
    }
    visiblePluginContextsRef.current = next;
  }, [state.workspace, loading, qualificationMode, plugins]);

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
      if (!alive) return;
      setHardwareStatus(event.payload);
      if (event.payload.state === 'connected') void bridge.pluginDispatchHostEvent('deviceDidConnect', { deviceInfo: { name: event.payload.model, type: 7, size: { columns: 4, rows: 2 } } }, workspaceRef.current.active_device_id).catch(() => undefined);
      else void bridge.pluginDispatchHostEvent('deviceDidDisconnect', {}, workspaceRef.current.active_device_id).catch(() => undefined);
    }).then((unlisten) => {
      if (alive) unlisteners.push(unlisten); else unlisten();
    }).catch(() => undefined);
    void listen<StreamDeckInputEvent>('opendeck://hardware-input', (event) => {
      if (!alive) return;
      const workspace = workspaceRef.current;
      const activePage = getActivePage(workspace);
      const presentation = resolveTouchStripPresentation(activePage, activeAppIdRef.current);
      if (event.payload.kind === 'keyUp' || event.payload.kind === 'dialUp') {
        const targetSlot = event.payload.kind === 'keyUp' ? activePage.slots.keys[event.payload.index] : activePage.slots.dials[event.payload.index];
        const binding = targetSlot ? effectiveDialBindings(targetSlot).press : undefined;
        if (event.payload.kind === 'keyUp' && targetSlot && binding && handleKeyLogicHardware('keyUp', targetSlot, binding)) return;
        if (targetSlot && binding && actionTreeContainsPlugin(binding)) void executeActionInstance(binding, targetSlot, 'press', 'hardware', workspace, event.payload);
        return;
      }
      if (event.payload.kind === 'keyDown') {
        const targetSlot = activePage.slots.keys[event.payload.index];
        const binding = targetSlot ? effectiveDialBindings(targetSlot).press : undefined;
        if (targetSlot && binding && handleKeyLogicHardware('keyDown', targetSlot, binding)) return;
      }
      const executions = resolveHardwareExecutions(event.payload, activePage, presentation);
      if (event.payload.kind === 'dialRotate' && executions.length > 0) {
        const firstSlot = findSlot(activePage, executions[0].selection);
        const firstBinding = firstSlot ? effectiveDialBindings(firstSlot)[executions[0].interaction] : undefined;
        if (firstSlot && firstBinding && actionTreeContainsPlugin(firstBinding)) {
          void executeBinding(firstSlot, executions[0].interaction, 'hardware', workspace, event.payload);
          return;
        }
      }
      for (const target of executions) {
        const targetSlot = findSlot(activePage, target.selection);
        if (targetSlot) void executeBinding(targetSlot, target.interaction, 'hardware', workspace, event.payload);
      }
    }).then((unlisten) => {
      if (alive) unlisteners.push(unlisten); else unlisten();
    }).catch(() => undefined);
    return () => {
      alive = false;
      for (const unlisten of unlisteners) unlisten();
    };
  }, [executeActionInstance, executeBinding, handleKeyLogicHardware, qualificationMode]);

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
      void bridge.streamdeckSyncWorkspace(runtimeWorkspace, activeAppId).catch((error) => {
        setHardwareStatus((current) => ({ ...current, state: 'ioError', message: `Hardware sync failed: ${String(error)}` }));
      });
    }, 120);
    return () => window.clearTimeout(timer);
  }, [runtimeWorkspace, activeAppId, loading, qualificationMode]);

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

  const selectedIsStackedDial = state.selection?.kind === 'dial' && Boolean(slot?.dialStack);
  const selectedIsWheelDial = state.selection?.kind === 'dial' && Boolean(slot?.actionWheel);

  useEffect(() => {
    setPreviewState('default');
    setSelectedInteraction(state.selection?.kind === 'touch' ? 'touch' : selectedIsWheelDial ? 'press' : selectedIsStackedDial ? 'rotateRight' : 'press');
  }, [state.selection?.slotId, state.selection?.kind, selectedIsStackedDial, selectedIsWheelDial]);

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
    if (id === 'editor.dialStack') {
      if (destinationSlot.kind !== 'dial') { setStatus('Dial Stack is only supported on dial controls.'); return; }
      if (destinationSlot.actionWheel) { setStatus('Remove the Action Wheel before creating a Dial Stack.'); return; }
      dispatch({ type: 'CREATE_DIAL_STACK', selection: destination });
      selectControl(destination);
      setSelectedInteraction('rotateRight');
      setStatus(`Created Dial Stack on dial ${destinationSlot.position + 1}.`);
      return;
    }
    if (id === 'editor.actionWheel') {
      if (destinationSlot.kind !== 'dial') { setStatus('Action Wheel is only supported on dial controls.'); return; }
      if (destinationSlot.actionWheel) { setStatus('This dial already has an Action Wheel.'); return; }
      if (destinationSlot.dialStack) { setStatus('Remove the Dial Stack before creating an Action Wheel.'); return; }
      dispatch({ type: 'CREATE_ACTION_WHEEL', selection: destination });
      selectControl(destination);
      setSelectedInteraction('press');
      setStatus(`Created Action Wheel on dial ${destinationSlot.position + 1}.`);
      return;
    }
    if (!supportsControl(id, destinationSlot.kind)) {
      setStatus(`${definition.label} is not supported on ${destinationSlot.kind} controls.`);
      return;
    }
    const interaction: Interaction = destinationSlot.kind === 'dial' && destinationSlot.actionWheel
      ? 'press'
      : interactionOverride ?? (destinationSlot.kind === 'touch' ? 'touch' : destinationSlot.kind === 'dial' && destinationSlot.dialStack ? 'rotateRight' : definition.defaultInteraction);
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

  async function importProfile(path: string) {
    const imported = await bridge.editorImportProfile(path);
    dispatch({ type: 'IMPORT_PROFILE', profile: imported });
    setStatus(`Imported profile ${imported.name}.`);
  }

  async function importBundledPluginProfile(uuid: string, profileName: string, activate: boolean) {
    const imported = await bridge.pluginImportBundledProfile(uuid, profileName);
    dispatch({ type: 'IMPORT_PROFILE', profile: imported, activate });
    setStatus(`${activate ? 'Installed and activated' : 'Installed'} plugin profile ${imported.name}.`);
  }

  async function installPlugin(path: string) {
    const plugin = await bridge.pluginInstall(path);
    let importedCount = 0;
    for (const bundled of plugin.profiles.filter((profile) => profile.autoInstall && profile.deviceType === 7)) {
      try {
        await importBundledPluginProfile(plugin.uuid, bundled.name, !bundled.dontAutoSwitchWhenInstalled);
        importedCount += 1;
      } catch (error) {
        setStatus(`Installed ${plugin.name}; bundled profile ${bundled.name} could not be installed: ${String(error)}`);
      }
    }
    await refreshPlugins();
    if (importedCount === 0) setStatus(`Installed ${plugin.name}. Select it and Activate to start the plugin.`);
  }

  async function setPluginEnabled(uuid: string, enabled: boolean) {
    await bridge.pluginSetEnabled(uuid, enabled);
    await refreshPlugins();
  }

  async function setPluginActive(uuid: string, active: boolean) {
    await bridge.pluginSetActive(uuid, active);
    await refreshPlugins();
  }

  async function restartPlugin(uuid: string) {
    await bridge.pluginRestart(uuid);
    await refreshPlugins();
  }

  async function removePlugin(uuid: string) {
    await bridge.pluginRemove(uuid);
    await refreshPlugins();
  }

  if (loading) return <div className="loading-screen">Loading OpenDeck editor…</div>;

  const canvasStyle = { '--opendeck-fit-scale': qualificationMode ? 1 : fitScale } as CSSProperties;

  return <div className="opendeck-viewport">
    {!qualificationMode && <WindowResizeFrame />}
    <div className="canonical-canvas" style={canvasStyle}>
      <div className="app-shell v231-shell">
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
    <div className={`app-body workspace-grid${section === 'profiles' || section === 'plugins' ? ' management-view' : ''}`}>
      <AppSidebar section={section} onSectionChange={(next) => {
        setSection(next);
        if (next === 'buttons') setActionMode('keys');
        if (next === 'dials' || next === 'touch') setActionMode('dials');
        if (next === 'plugins') { setPanel('none'); void refreshPlugins(); void scanMarketplace(); }
        if (next === 'settings') setPanel('settings');
      }} />
      <main className="main-editor" data-layout-region="main">
        {section === 'profiles' ? <ProfilesWorkspace
          workspace={state.workspace}
          onActivate={(profileId) => { dispatch({ type: 'SET_ACTIVE_PROFILE', profileId }); setStatus(`Activated ${state.workspace.profiles.find((candidate) => candidate.id === profileId)?.name ?? 'profile'} on Stream Deck +.`); }}
          onCreate={() => dispatch({ type: 'CREATE_PROFILE' })}
          onRename={(profileId, name) => dispatch({ type: 'RENAME_PROFILE', profileId, name })}
          onDuplicate={(profileId) => dispatch({ type: 'DUPLICATE_PROFILE', profileId })}
          onDelete={(profileId) => dispatch({ type: 'DELETE_PROFILE', profileId })}
          onExport={(selectedProfile, path) => bridge.editorExportProfile(selectedProfile, path)}
          onImport={importProfile}
        /> : section === 'plugins' ? <PluginManager
          plugins={plugins}
          hostStatus={pluginHostStatus}
          marketplaceItems={marketItems}
          onScan={scanMarketplace}
          onOpenMarketplace={() => bridge.openExternal('https://marketplace.elgato.com')}
          onInstall={installPlugin}
          onSetEnabled={setPluginEnabled}
          onSetActive={setPluginActive}
          onRestart={restartPlugin}
          onRemove={removePlugin}
          onImportBundledProfile={importBundledPluginProfile}
        /> : <section className="editor-column" style={editorStyle}>
          <DeviceEditor
            page={runtimePage}
            selection={state.selection}
            assetPreviews={runtimeAssetPreviews}
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
            onDialStackAdd={() => dispatch({ type: 'ADD_DIAL_STACK_ENTRY' })}
            onDialStackSelect={(entryId) => dispatch({ type: 'SET_DIAL_STACK_ACTIVE', entryId })}
            onDialStackRename={(entryId, label) => dispatch({ type: 'RENAME_DIAL_STACK_ENTRY', entryId, label })}
            onDialStackMove={(entryId, direction) => dispatch({ type: 'MOVE_DIAL_STACK_ENTRY', entryId, direction })}
            onDialStackRemove={(entryId) => dispatch({ type: 'REMOVE_DIAL_STACK_ENTRY', entryId })}
            onDialStackRemoveStack={() => dispatch({ type: 'REMOVE_DIAL_STACK' })}
            onActionWheelAdd={() => dispatch({ type: 'ADD_ACTION_WHEEL_ENTRY' })}
            onActionWheelSelect={(entryId) => dispatch({ type: 'SET_ACTION_WHEEL_ACTIVE', entryId })}
            onActionWheelRename={(entryId, label) => dispatch({ type: 'RENAME_ACTION_WHEEL_ENTRY', entryId, label })}
            onActionWheelMove={(entryId, direction) => dispatch({ type: 'MOVE_ACTION_WHEEL_ENTRY', entryId, direction })}
            onActionWheelRemove={(entryId) => dispatch({ type: 'REMOVE_ACTION_WHEEL_ENTRY', entryId })}
            onActionWheelRemoveWheel={() => dispatch({ type: 'REMOVE_ACTION_WHEEL' })}
            touchStrip={page.touchStrip}
            onTouchMode={(mode) => dispatch({ type: 'SET_TOUCH_STRIP_MODE', mode })}
            onTouchFallback={(presentation) => dispatch({ type: 'SET_TOUCH_STRIP_FALLBACK', presentation })}
            onTouchRules={(rules) => dispatch({ type: 'SET_TOUCH_STRIP_RULES', rules })}
          />
        </section>}
      </main>
      {section !== 'profiles' && section !== 'plugins' && <ActionLibrary
        mode={actionMode}
        collapsed={preferences.actionPanelCollapsed}
        onModeChange={setActionMode}
        onToggleCollapsed={() => dispatch({ type: 'UPDATE_PREFERENCES', patch: { actionPanelCollapsed: !preferences.actionPanelCollapsed } })}
        onChoose={(id) => assignAction(id, state.selection, selectedInteraction)}
        externalActions={externalActions}
        qualificationCatalogSize={qualificationMode && qualificationPhase === 'performance' ? 5000 : 0}
      />}
    </div>
    <QualificationHarness enabled={qualificationMode} phase={qualificationPhase} workspace={state.workspace} startedAtMs={qualification.startedAtMs} tauriSetupMs={qualification.tauriSetupMs} />
    <div className="editor-toast" role="status" aria-live="polite">{status !== 'Ready' ? status : ''}</div>

    {assetRole && <div className="overlay asset-overlay" onMouseDown={() => setAssetRole(null)}><section className="modal asset-modal" onMouseDown={(e) => e.stopPropagation()}><AssetBrowser assets={state.workspace.assets} role={assetRole.role} loadPreviews={loadAssetPreviews} onPick={(assetId, role) => { const patch = role === 'icon' ? { iconAssetId: assetId } : { backgroundAssetId: assetId }; dispatch(assetRole.target === 'active' ? { type: 'UPDATE_STATE', stateName: 'active', patch } : { type: 'UPDATE_APPEARANCE', patch }); setAssetRole(null); }} onImport={importAsset} onClose={() => setAssetRole(null)} /></section></div>}

    {panel !== 'none' && <div className="overlay" onMouseDown={() => setPanel('none')}><section className="modal compact-modal" onMouseDown={(e) => e.stopPropagation()}><button className="close" aria-label="Close" onClick={() => setPanel('none')}>×</button>{panel === 'connections' ? <><h2>Connections</h2><div className="connection-card"><h3>OBS Studio</h3><div className="row"><input value={obsHost} onChange={(e) => setObsHost(e.target.value)} aria-label="OBS host"/><input type="number" value={obsPort} onChange={(e) => setObsPort(Number(e.target.value))} aria-label="OBS port"/></div><input type="password" value={obsPassword} onChange={(e) => setObsPassword(e.target.value)} aria-label="OBS password" placeholder="WebSocket password"/><button onClick={() => void connectObs()}>Connect</button><p>{obsStatus}</p>{scenes.length > 0 && <small>{scenes.length} scenes available</small>}</div><div className="connection-card"><h3>Twitch</h3>{twitchIdentity ? <p>Signed in as <strong>{twitchIdentity.login}</strong></p> : <><button disabled={twitchAuthMessage === 'Starting Twitch sign-in…' || twitchAuthMessage === 'Waiting for Twitch…'} onClick={() => void beginTwitch()}>Sign in with Twitch</button>{twitchAuthMessage && <p>{twitchAuthMessage}</p>}{twitchCode && <><p>Code: <code>{twitchCode.user_code}</code></p><button className="subtle" onClick={cancelTwitchSignIn}>Cancel Twitch sign-in</button></>}</>}</div></> : panel === 'marketplace' ? <><h2>Elgato Marketplace</h2><div className="row"><button onClick={() => void bridge.openExternal('https://marketplace.elgato.com')}>Open Marketplace</button><button onClick={() => void scanMarketplace()}>Scan Downloads</button></div><div className="market-list">{marketItems.length ? marketItems.map((item) => <article key={item.path}><strong>{item.name}</strong><span>{item.kind}</span><small>{item.path}</small></article>) : <p>No compatible downloaded items found.</p>}</div></> : <><h2>Settings</h2><div className="connection-card"><h3>OpenDeck+ 2.0.44</h3><p>Render-parity mode keeps DragonGlass visual geometry and responsiveness gates active.</p><div className="row"><button onClick={() => { setPanel('connections'); }}>Connections</button><button onClick={() => { setPanel('marketplace'); void scanMarketplace(); }}>Marketplace</button></div></div></>}</section></div>}
      </div>
    </div>
  </div>;
}
