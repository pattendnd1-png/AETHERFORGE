import { allSlots, createId, createPage, createProfile, cloneWorkspace, findSlot, getActivePage, getActiveProfile, type ActionInstance, type Appearance, type AppearanceOverride, type AssetRecord, type ControlSelection, type EditorPreferences, type Interaction, type Profile, type TouchStripMode, type TouchStripPresentation, type TouchStripRule, type Workspace } from '../model/workspace';

const HISTORY_LIMIT = 50;

export interface EditorState {
  workspace: Workspace;
  selection: ControlSelection | null;
  past: Workspace[];
  future: Workspace[];
}

export type EditorAction =
  | { type: 'SELECT_CONTROL'; selection: ControlSelection | null }
  | { type: 'REPLACE_WORKSPACE'; workspace: Workspace }
  | { type: 'ASSIGN_ACTION'; interaction: Interaction; action: ActionInstance }
  | { type: 'ASSIGN_ACTION_TO_CONTROL'; selection: ControlSelection; interaction: Interaction; action: ActionInstance; title: string }
  | { type: 'REMOVE_ACTION'; interaction: Interaction }
  | { type: 'UPDATE_ACTION_CONFIG'; interaction: Interaction; patch: Record<string, unknown> }
  | { type: 'UPDATE_APPEARANCE'; patch: Partial<Appearance> }
  | { type: 'UPDATE_STATE'; stateName: string; patch: AppearanceOverride }
  | { type: 'RESET_STATE'; stateName: string }
  | { type: 'ADD_PAGE'; name?: string }
  | { type: 'DUPLICATE_PAGE' }
  | { type: 'RENAME_PAGE'; pageId: string; name: string }
  | { type: 'DELETE_PAGE' }
  | { type: 'SET_ACTIVE_PAGE'; pageId: string }
  | { type: 'CREATE_PROFILE'; name?: string }
  | { type: 'RENAME_PROFILE'; profileId: string; name: string }
  | { type: 'DUPLICATE_PROFILE'; profileId: string }
  | { type: 'DELETE_PROFILE'; profileId: string }
  | { type: 'SET_ACTIVE_PROFILE'; profileId: string }
  | { type: 'MOVE_CONTROL'; source: ControlSelection; destination: ControlSelection }
  | { type: 'COPY_CONTROL'; source: ControlSelection; destination: ControlSelection }
  | { type: 'UPSERT_ASSET'; asset: AssetRecord }
  | { type: 'SET_ASSETS'; assets: AssetRecord[] }
  | { type: 'UPDATE_PREFERENCES'; patch: Partial<EditorPreferences> }
  | { type: 'SET_TOUCH_STRIP_MODE'; mode: TouchStripMode }
  | { type: 'SET_TOUCH_STRIP_FALLBACK'; presentation: TouchStripPresentation }
  | { type: 'SET_TOUCH_STRIP_RULES'; rules: TouchStripRule[] }
  | { type: 'CREATE_DIAL_STACK'; selection: ControlSelection }
  | { type: 'ADD_DIAL_STACK_ENTRY'; label?: string }
  | { type: 'SET_DIAL_STACK_ACTIVE'; entryId: string }
  | { type: 'RENAME_DIAL_STACK_ENTRY'; entryId: string; label: string }
  | { type: 'MOVE_DIAL_STACK_ENTRY'; entryId: string; direction: -1 | 1 }
  | { type: 'REMOVE_DIAL_STACK_ENTRY'; entryId: string }
  | { type: 'REMOVE_DIAL_STACK' }
  | { type: 'CYCLE_DIAL_STACK'; selection: ControlSelection }
  | { type: 'CREATE_ACTION_WHEEL'; selection: ControlSelection }
  | { type: 'ADD_ACTION_WHEEL_ENTRY'; label?: string }
  | { type: 'SET_ACTION_WHEEL_ACTIVE'; entryId: string }
  | { type: 'RENAME_ACTION_WHEEL_ENTRY'; entryId: string; label: string }
  | { type: 'MOVE_ACTION_WHEEL_ENTRY'; entryId: string; direction: -1 | 1 }
  | { type: 'REMOVE_ACTION_WHEEL_ENTRY'; entryId: string }
  | { type: 'REMOVE_ACTION_WHEEL' }
  | { type: 'STEP_ACTION_WHEEL'; selection: ControlSelection; direction: -1 | 1 }
  | { type: 'UNDO' }
  | { type: 'REDO' };

export function createEditorState(workspace: Workspace): EditorState {
  return { workspace: cloneWorkspace(workspace), selection: null, past: [], future: [] };
}

export function selectedSlot(state: EditorState) {
  if (!state.selection) return undefined;
  return findSlot(getActivePage(state.workspace), state.selection);
}

export function canUndo(state: EditorState): boolean { return state.past.length > 0; }
export function canRedo(state: EditorState): boolean { return state.future.length > 0; }

function commit(state: EditorState, mutate: (workspace: Workspace) => void): EditorState {
  const next = cloneWorkspace(state.workspace);
  mutate(next);
  return {
    ...state,
    workspace: next,
    past: [...state.past.slice(-(HISTORY_LIMIT - 1)), cloneWorkspace(state.workspace)],
    future: [],
  };
}

function slotFor(workspace: Workspace, selection: ControlSelection) {
  return findSlot(getActivePage(workspace), selection);
}

function copyConfig(source: ReturnType<typeof slotFor>, destination: ReturnType<typeof slotFor>): void {
  if (!source || !destination) return;
  destination.bindings = structuredClone(source.bindings);
  destination.appearance = structuredClone(source.appearance);
  destination.states = structuredClone(source.states);
  destination.folderTarget = source.folderTarget;
  destination.dialStack = source.dialStack ? {
    ...structuredClone(source.dialStack),
    entries: source.dialStack.entries.map((entry) => ({
      ...structuredClone(entry),
      id: createId('dial-stack-entry'),
    })),
  } : null;
  destination.actionWheel = source.actionWheel ? {
    ...structuredClone(source.actionWheel),
    entries: source.actionWheel.entries.map((entry) => ({
      ...structuredClone(entry),
      id: createId('action-wheel-entry'),
    })),
  } : null;
}

function activeStackEntry(slot: ReturnType<typeof slotFor>) {
  const stack = slot?.dialStack;
  if (!stack || stack.entries.length === 0) return undefined;
  const index = Math.min(Math.max(0, stack.activeIndex), stack.entries.length - 1);
  return stack.entries[index];
}

function activeWheelEntry(slot: ReturnType<typeof slotFor>) {
  const wheel = slot?.actionWheel;
  if (!wheel || wheel.entries.length === 0) return undefined;
  const index = Math.min(Math.max(0, wheel.activeIndex), wheel.entries.length - 1);
  return wheel.entries[index];
}

function editableBindings(slot: ReturnType<typeof slotFor>) {
  return activeWheelEntry(slot)?.bindings ?? activeStackEntry(slot)?.bindings ?? slot?.bindings;
}

function bindingMaps(slot: ReturnType<typeof slotFor>): Array<NonNullable<ReturnType<typeof editableBindings>>> {
  if (!slot) return [];
  return [
    slot.bindings,
    ...(slot.dialStack?.entries.map((entry) => entry.bindings) ?? []),
    ...(slot.actionWheel?.entries.map((entry) => entry.bindings) ?? []),
  ];
}

function remapFolderPageReferences(profile: Profile, pageIds: Map<string, string>): void {
  for (const page of profile.pages) {
    for (const slot of allSlots(page)) {
      if (slot.folderTarget) slot.folderTarget = pageIds.get(slot.folderTarget) ?? null;
      for (const bindings of bindingMaps(slot)) for (const binding of Object.values(bindings)) {
        if (!binding || binding.definitionId !== 'editor.folder') continue;
        const target = binding.config.pageId;
        if (typeof target === 'string' && target) {
          binding.config = { ...binding.config, pageId: pageIds.get(target) ?? '' };
        }
      }
    }
  }
}

function clearFolderPageReferences(profile: Profile, pageId: string): void {
  for (const page of profile.pages) {
    for (const slot of allSlots(page)) {
      if (slot.folderTarget === pageId) slot.folderTarget = null;
      for (const bindings of bindingMaps(slot)) for (const binding of Object.values(bindings)) {
        if (!binding || binding.definitionId !== 'editor.folder') continue;
        if (binding.config.pageId === pageId) binding.config = { ...binding.config, pageId: '' };
      }
    }
  }
}

function clearProfileReferences(workspace: Workspace, profileId: string): void {
  for (const profile of workspace.profiles) {
    for (const page of profile.pages) {
      for (const slot of allSlots(page)) {
        for (const bindings of bindingMaps(slot)) for (const binding of Object.values(bindings)) {
          if (!binding || binding.definitionId !== 'editor.switchProfile') continue;
          if (binding.config.profileId === profileId) binding.config = { ...binding.config, profileId: '' };
        }
      }
    }
  }
}

export function editorReducer(state: EditorState, action: EditorAction): EditorState {
  switch (action.type) {
    case 'SELECT_CONTROL':
      return { ...state, selection: action.selection };
    case 'REPLACE_WORKSPACE':
      return createEditorState(action.workspace);
    case 'ASSIGN_ACTION':
      return commit(state, (workspace) => {
        const slot = state.selection ? slotFor(workspace, state.selection) : undefined;
        const bindings = editableBindings(slot);
        if (bindings) bindings[action.interaction] = structuredClone(action.action);
      });
    case 'ASSIGN_ACTION_TO_CONTROL':
      return commit(state, (workspace) => {
        const slot = slotFor(workspace, action.selection);
        if (!slot) return;
        const bindings = editableBindings(slot);
        if (!bindings) return;
        bindings[action.interaction] = structuredClone(action.action);
        const stackEntry = activeStackEntry(slot);
        const wheelEntry = activeWheelEntry(slot);
        if (stackEntry) {
          if (!stackEntry.label.trim() || /^Entry \d+$/.test(stackEntry.label)) stackEntry.label = action.title;
        } else if (wheelEntry) {
          if (!wheelEntry.label.trim() || /^Action \d+$/.test(wheelEntry.label)) wheelEntry.label = action.title;
        } else {
          slot.appearance.title = action.title;
        }
        if (action.action.definitionId !== 'editor.folder') slot.folderTarget = null;
      });
    case 'REMOVE_ACTION':
      return commit(state, (workspace) => {
        const slot = state.selection ? slotFor(workspace, state.selection) : undefined;
        if (!slot) return;
        const bindings = editableBindings(slot);
        if (!bindings) return;
        if (bindings[action.interaction]?.definitionId === 'editor.folder') slot.folderTarget = null;
        delete bindings[action.interaction];
      });
    case 'UPDATE_ACTION_CONFIG':
      return commit(state, (workspace) => {
        const slot = state.selection ? slotFor(workspace, state.selection) : undefined;
        const binding = editableBindings(slot)?.[action.interaction];
        if (binding) {
          binding.config = { ...binding.config, ...action.patch };
          if (slot && binding.definitionId === 'editor.folder' && typeof action.patch.pageId === 'string') {
            slot.folderTarget = action.patch.pageId || null;
          }
        }
      });
    case 'UPDATE_APPEARANCE':
      return commit(state, (workspace) => {
        const slot = state.selection ? slotFor(workspace, state.selection) : undefined;
        if (slot) slot.appearance = { ...slot.appearance, ...action.patch };
      });
    case 'UPDATE_STATE':
      return commit(state, (workspace) => {
        const slot = state.selection ? slotFor(workspace, state.selection) : undefined;
        if (slot) slot.states[action.stateName] = { ...(slot.states[action.stateName] ?? {}), ...action.patch };
      });
    case 'RESET_STATE':
      return commit(state, (workspace) => {
        const slot = state.selection ? slotFor(workspace, state.selection) : undefined;
        if (slot) delete slot.states[action.stateName];
      });
    case 'ADD_PAGE':
      return commit(state, (workspace) => {
        const profile = getActiveProfile(workspace);
        const page = createPage(action.name ?? `Page ${profile.pages.length + 1}`);
        profile.pages.push(page);
        profile.active_page_id = page.id;
      });
    case 'DUPLICATE_PAGE':
      return commit(state, (workspace) => {
        const profile = getActiveProfile(workspace);
        const source = getActivePage(workspace);
        const page = createPage(`${source.name} Copy`);
        page.slots.keys.forEach((slot, i) => copyConfig(source.slots.keys[i], slot));
        page.slots.dials.forEach((slot, i) => copyConfig(source.slots.dials[i], slot));
        page.slots.touch_regions.forEach((slot, i) => copyConfig(source.slots.touch_regions[i], slot));
        copyConfig(source.touchStrip.unifiedSlot, page.touchStrip.unifiedSlot);
        page.touchStrip.mode = source.touchStrip.mode;
        page.touchStrip.adaptiveFallback = source.touchStrip.adaptiveFallback;
        page.touchStrip.adaptiveRules = structuredClone(source.touchStrip.adaptiveRules);
        profile.pages.push(page);
        profile.active_page_id = page.id;
      });
    case 'RENAME_PAGE':
      return commit(state, (workspace) => {
        const profile = getActiveProfile(workspace);
        const page = profile.pages.find((candidate) => candidate.id === action.pageId);
        if (page && action.name.trim()) page.name = action.name.trim();
      });
    case 'DELETE_PAGE': {
      if (getActiveProfile(state.workspace).pages.length <= 1) return state;
      return commit(state, (workspace) => {
        const profile = getActiveProfile(workspace);
        const deletedPageId = profile.active_page_id;
        profile.pages = profile.pages.filter((page) => page.id !== deletedPageId);
        profile.active_page_id = profile.pages[0].id;
        clearFolderPageReferences(profile, deletedPageId);
      });
    }
    case 'SET_ACTIVE_PAGE':
      return commit(state, (workspace) => {
        const profile = getActiveProfile(workspace);
        if (profile.pages.some((page) => page.id === action.pageId)) profile.active_page_id = action.pageId;
      });
    case 'CREATE_PROFILE':
      return commit(state, (workspace) => {
        const profile = createProfile(action.name ?? `Profile ${workspace.profiles.length + 1}`, workspace.active_device_id);
        workspace.profiles.push(profile);
        workspace.active_profile_id = profile.id;
      });
    case 'RENAME_PROFILE':
      return commit(state, (workspace) => {
        const profile = workspace.profiles.find((candidate) => candidate.id === action.profileId);
        if (profile && action.name.trim()) profile.name = action.name.trim();
      });
    case 'DUPLICATE_PROFILE':
      return commit(state, (workspace) => {
        const source = workspace.profiles.find((candidate) => candidate.id === action.profileId);
        if (!source) return;
        const profile = createProfile(`${source.name} Copy`, source.device_id);
        profile.app_match = structuredClone(source.app_match);
        const pageIds = new Map<string, string>();
        profile.pages = source.pages.map((sourcePage) => {
          const page = createPage(sourcePage.name);
          pageIds.set(sourcePage.id, page.id);
          page.parentFolderId = sourcePage.parentFolderId;
          page.slots.keys.forEach((slot, i) => copyConfig(sourcePage.slots.keys[i], slot));
          page.slots.dials.forEach((slot, i) => copyConfig(sourcePage.slots.dials[i], slot));
          page.slots.touch_regions.forEach((slot, i) => copyConfig(sourcePage.slots.touch_regions[i], slot));
          copyConfig(sourcePage.touchStrip.unifiedSlot, page.touchStrip.unifiedSlot);
          page.touchStrip.mode = sourcePage.touchStrip.mode;
          page.touchStrip.adaptiveFallback = sourcePage.touchStrip.adaptiveFallback;
          page.touchStrip.adaptiveRules = structuredClone(sourcePage.touchStrip.adaptiveRules);
          return page;
        });
        remapFolderPageReferences(profile, pageIds);
        profile.active_page_id = pageIds.get(source.active_page_id) ?? profile.pages[0].id;
        workspace.profiles.push(profile);
        workspace.active_profile_id = profile.id;
      });
    case 'DELETE_PROFILE': {
      if (state.workspace.profiles.length <= 1) return state;
      return commit(state, (workspace) => {
        workspace.profiles = workspace.profiles.filter((profile) => profile.id !== action.profileId);
        clearProfileReferences(workspace, action.profileId);
        if (workspace.active_profile_id === action.profileId) workspace.active_profile_id = workspace.profiles[0].id;
      });
    }
    case 'SET_ACTIVE_PROFILE':
      return commit(state, (workspace) => {
        const profile = workspace.profiles.find((candidate) => candidate.id === action.profileId);
        if (profile) { workspace.active_profile_id = profile.id; workspace.active_device_id = profile.device_id; }
      });
    case 'MOVE_CONTROL':
      return commit(state, (workspace) => {
        const source = slotFor(workspace, action.source);
        const destination = slotFor(workspace, action.destination);
        if (!source || !destination) return;
        const sourceConfig = { bindings: structuredClone(source.bindings), appearance: structuredClone(source.appearance), states: structuredClone(source.states), folderTarget: source.folderTarget, dialStack: structuredClone(source.dialStack), actionWheel: structuredClone(source.actionWheel) };
        const destinationConfig = { bindings: structuredClone(destination.bindings), appearance: structuredClone(destination.appearance), states: structuredClone(destination.states), folderTarget: destination.folderTarget, dialStack: structuredClone(destination.dialStack), actionWheel: structuredClone(destination.actionWheel) };
        Object.assign(source, destinationConfig);
        Object.assign(destination, sourceConfig);
      });
    case 'COPY_CONTROL':
      return commit(state, (workspace) => {
        copyConfig(slotFor(workspace, action.source), slotFor(workspace, action.destination));
      });
    case 'UPSERT_ASSET':
      return commit(state, (workspace) => {
        workspace.assets = [...workspace.assets.filter((asset) => asset.id !== action.asset.id), structuredClone(action.asset)];
      });
    case 'SET_ASSETS':
      return commit(state, (workspace) => {
        workspace.assets = structuredClone(action.assets);
      });
    case 'UPDATE_PREFERENCES': {
      const workspace = cloneWorkspace(state.workspace);
      workspace.preferences = { ...workspace.preferences, ...action.patch };
      return { ...state, workspace };
    }
    case 'SET_TOUCH_STRIP_MODE':
      return commit(state, (workspace) => { getActivePage(workspace).touchStrip.mode = action.mode; });
    case 'SET_TOUCH_STRIP_FALLBACK':
      return commit(state, (workspace) => { getActivePage(workspace).touchStrip.adaptiveFallback = action.presentation; });
    case 'SET_TOUCH_STRIP_RULES':
      return commit(state, (workspace) => { getActivePage(workspace).touchStrip.adaptiveRules = structuredClone(action.rules); });
    case 'CREATE_DIAL_STACK':
      return commit(state, (workspace) => {
        const slot = slotFor(workspace, action.selection);
        if (!slot || slot.kind !== 'dial' || slot.dialStack || slot.actionWheel) return;
        slot.dialStack = {
          behavior: 'pressCycle',
          activeIndex: 0,
          entries: [{
            id: createId('dial-stack-entry'),
            label: slot.appearance.title.trim() || 'Entry 1',
            bindings: structuredClone(slot.bindings),
          }],
        };
      });
    case 'ADD_DIAL_STACK_ENTRY':
      return commit(state, (workspace) => {
        const slot = state.selection ? slotFor(workspace, state.selection) : undefined;
        const stack = slot?.dialStack;
        if (!slot || slot.kind !== 'dial' || !stack) return;
        const entry = { id: createId('dial-stack-entry'), label: action.label?.trim() || `Entry ${stack.entries.length + 1}`, bindings: {} };
        stack.entries.push(entry);
        stack.activeIndex = stack.entries.length - 1;
      });
    case 'SET_DIAL_STACK_ACTIVE':
      return commit(state, (workspace) => {
        const slot = state.selection ? slotFor(workspace, state.selection) : undefined;
        const stack = slot?.dialStack;
        if (!stack) return;
        const index = stack.entries.findIndex((entry) => entry.id === action.entryId);
        if (index >= 0) stack.activeIndex = index;
      });
    case 'RENAME_DIAL_STACK_ENTRY':
      return commit(state, (workspace) => {
        const slot = state.selection ? slotFor(workspace, state.selection) : undefined;
        const entry = slot?.dialStack?.entries.find((candidate) => candidate.id === action.entryId);
        const label = action.label.trim();
        if (entry && label) entry.label = label;
      });
    case 'MOVE_DIAL_STACK_ENTRY':
      return commit(state, (workspace) => {
        const slot = state.selection ? slotFor(workspace, state.selection) : undefined;
        const stack = slot?.dialStack;
        if (!stack) return;
        const index = stack.entries.findIndex((entry) => entry.id === action.entryId);
        const destination = index + action.direction;
        if (index < 0 || destination < 0 || destination >= stack.entries.length) return;
        const [entry] = stack.entries.splice(index, 1);
        stack.entries.splice(destination, 0, entry);
        stack.activeIndex = destination;
      });
    case 'REMOVE_DIAL_STACK_ENTRY':
      return commit(state, (workspace) => {
        const slot = state.selection ? slotFor(workspace, state.selection) : undefined;
        const stack = slot?.dialStack;
        if (!slot || !stack) return;
        const index = stack.entries.findIndex((entry) => entry.id === action.entryId);
        if (index < 0) return;
        stack.entries.splice(index, 1);
        if (stack.entries.length === 0) { slot.dialStack = null; return; }
        stack.activeIndex = Math.min(stack.activeIndex, stack.entries.length - 1);
      });
    case 'REMOVE_DIAL_STACK':
      return commit(state, (workspace) => {
        const slot = state.selection ? slotFor(workspace, state.selection) : undefined;
        if (slot?.kind === 'dial') slot.dialStack = null;
      });
    case 'CYCLE_DIAL_STACK':
      return commit(state, (workspace) => {
        const slot = slotFor(workspace, action.selection);
        const stack = slot?.dialStack;
        if (!stack || stack.entries.length === 0) return;
        stack.activeIndex = (Math.min(Math.max(0, stack.activeIndex), stack.entries.length - 1) + 1) % stack.entries.length;
      });
    case 'CREATE_ACTION_WHEEL':
      return commit(state, (workspace) => {
        const slot = slotFor(workspace, action.selection);
        if (!slot || slot.kind !== 'dial' || slot.dialStack || slot.actionWheel) return;
        const existingPress = slot.bindings.press ? structuredClone(slot.bindings.press) : undefined;
        slot.actionWheel = {
          behavior: 'rotateSelectPressExecute',
          activeIndex: 0,
          entries: [{
            id: createId('action-wheel-entry'),
            label: slot.appearance.title.trim() || 'Action 1',
            bindings: existingPress ? { press: existingPress } : {},
          }],
        };
      });
    case 'ADD_ACTION_WHEEL_ENTRY':
      return commit(state, (workspace) => {
        const slot = state.selection ? slotFor(workspace, state.selection) : undefined;
        const wheel = slot?.actionWheel;
        if (!slot || slot.kind !== 'dial' || !wheel) return;
        wheel.entries.push({
          id: createId('action-wheel-entry'),
          label: action.label?.trim() || `Action ${wheel.entries.length + 1}`,
          bindings: {},
        });
        wheel.activeIndex = wheel.entries.length - 1;
      });
    case 'SET_ACTION_WHEEL_ACTIVE':
      return commit(state, (workspace) => {
        const slot = state.selection ? slotFor(workspace, state.selection) : undefined;
        const wheel = slot?.actionWheel;
        if (!wheel) return;
        const index = wheel.entries.findIndex((entry) => entry.id === action.entryId);
        if (index >= 0) wheel.activeIndex = index;
      });
    case 'RENAME_ACTION_WHEEL_ENTRY':
      return commit(state, (workspace) => {
        const slot = state.selection ? slotFor(workspace, state.selection) : undefined;
        const entry = slot?.actionWheel?.entries.find((candidate) => candidate.id === action.entryId);
        const label = action.label.trim();
        if (entry && label) entry.label = label;
      });
    case 'MOVE_ACTION_WHEEL_ENTRY':
      return commit(state, (workspace) => {
        const slot = state.selection ? slotFor(workspace, state.selection) : undefined;
        const wheel = slot?.actionWheel;
        if (!wheel) return;
        const activeId = wheel.entries[Math.min(Math.max(0, wheel.activeIndex), wheel.entries.length - 1)]?.id;
        const index = wheel.entries.findIndex((entry) => entry.id === action.entryId);
        const destination = index + action.direction;
        if (index < 0 || destination < 0 || destination >= wheel.entries.length) return;
        const [entry] = wheel.entries.splice(index, 1);
        wheel.entries.splice(destination, 0, entry);
        wheel.activeIndex = Math.max(0, wheel.entries.findIndex((candidate) => candidate.id === activeId));
      });
    case 'REMOVE_ACTION_WHEEL_ENTRY':
      return commit(state, (workspace) => {
        const slot = state.selection ? slotFor(workspace, state.selection) : undefined;
        const wheel = slot?.actionWheel;
        if (!slot || !wheel) return;
        const index = wheel.entries.findIndex((entry) => entry.id === action.entryId);
        if (index < 0) return;
        wheel.entries.splice(index, 1);
        if (wheel.entries.length === 0) { slot.actionWheel = null; return; }
        wheel.activeIndex = Math.min(wheel.activeIndex, wheel.entries.length - 1);
      });
    case 'REMOVE_ACTION_WHEEL':
      return commit(state, (workspace) => {
        const slot = state.selection ? slotFor(workspace, state.selection) : undefined;
        if (slot?.kind === 'dial') slot.actionWheel = null;
      });
    case 'STEP_ACTION_WHEEL':
      return commit(state, (workspace) => {
        const slot = slotFor(workspace, action.selection);
        const wheel = slot?.actionWheel;
        if (!wheel || wheel.entries.length === 0) return;
        const current = Math.min(Math.max(0, wheel.activeIndex), wheel.entries.length - 1);
        wheel.activeIndex = (current + action.direction + wheel.entries.length) % wheel.entries.length;
      });
    case 'UNDO': {
      const previous = state.past.at(-1);
      if (!previous) return state;
      return {
        ...state,
        workspace: cloneWorkspace(previous),
        past: state.past.slice(0, -1),
        future: [cloneWorkspace(state.workspace), ...state.future].slice(0, HISTORY_LIMIT),
      };
    }
    case 'REDO': {
      const next = state.future[0];
      if (!next) return state;
      return {
        ...state,
        workspace: cloneWorkspace(next),
        past: [...state.past, cloneWorkspace(state.workspace)].slice(-HISTORY_LIMIT),
        future: state.future.slice(1),
      };
    }
  }
}
