export type ControlKind = 'key' | 'dial' | 'touch';
export type Interaction = 'press' | 'longPress' | 'rotateLeft' | 'rotateRight' | 'touch';
export type HorizontalAlign = 'left' | 'center' | 'right';
export type VerticalAlign = 'top' | 'middle' | 'bottom';
export type FitMode = 'contain' | 'cover' | 'stretch';

export interface ActionInstance {
  definitionId: string;
  config: Record<string, unknown>;
}

export interface Appearance {
  title: string;
  titleVisible: boolean;
  fontFamily: string;
  fontSize: number;
  fontWeight: number;
  textColor: string;
  horizontalAlign: HorizontalAlign;
  verticalAlign: VerticalAlign;
  iconAssetId: string | null;
  backgroundAssetId: string | null;
  backgroundColor: string;
  fitMode: FitMode;
  iconOpacity: number;
  titleOffsetX: number;
  titleOffsetY: number;
}

export type AppearanceOverride = Partial<Appearance>;

export interface ControlSlot {
  id: string;
  kind: ControlKind;
  position: number;
  bindings: Partial<Record<Interaction, ActionInstance>>;
  appearance: Appearance;
  states: Record<string, AppearanceOverride>;
  folderTarget: string | null;
}

export interface PageSlots {
  keys: ControlSlot[];
  dials: ControlSlot[];
  touch_regions: ControlSlot[];
}

export interface Page {
  id: string;
  name: string;
  slots: PageSlots;
  parentFolderId: string | null;
}

export interface Profile {
  id: string;
  name: string;
  device_id: string;
  pages: Page[];
  active_page_id: string;
  app_match: string[];
}

export interface EditorPreferences {
  actionPanelWidth: number;
  inspectorHeight: number;
  actionPanelCollapsed: boolean;
  inspectorCollapsed: boolean;
  zoom: number;
}

export interface AssetRecord {
  id: string;
  name: string;
  path: string;
  source: 'imported' | 'icon-pack' | 'marketplace' | 'builtin';
  mime: string;
  sha256: string;
}

export interface Workspace {
  schema_version: 1;
  active_device_id: string;
  active_profile_id: string;
  profiles: Profile[];
  assets: AssetRecord[];
  preferences: EditorPreferences;
}

export interface ControlSelection {
  kind: ControlKind;
  slotId: string;
}

export interface WorkspaceLoadResult {
  workspace: Workspace;
  source: 'primary' | 'backup' | 'default';
  warning: string | null;
}

let fallbackId = 0;

export function createId(prefix: string): string {
  if (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function') {
    return `${prefix}-${crypto.randomUUID()}`;
  }
  fallbackId += 1;
  return `${prefix}-${fallbackId}`;
}

export function resolveAppearance(slot: ControlSlot, stateName?: string | null): Appearance {
  if (!stateName || stateName === 'default') return { ...slot.appearance };
  return { ...slot.appearance, ...(slot.states[stateName] ?? {}) };
}

export function createDefaultAppearance(title = ''): Appearance {
  return {
    title,
    titleVisible: true,
    fontFamily: 'Inter, system-ui, sans-serif',
    fontSize: 14,
    fontWeight: 500,
    textColor: '#ffffff',
    horizontalAlign: 'center',
    verticalAlign: 'bottom',
    iconAssetId: null,
    backgroundAssetId: null,
    backgroundColor: '#16181d',
    fitMode: 'contain',
    iconOpacity: 1,
    titleOffsetX: 0,
    titleOffsetY: 0,
  };
}

export function createControlSlot(kind: ControlKind, position: number): ControlSlot {
  return {
    id: createId(kind),
    kind,
    position,
    bindings: {},
    appearance: createDefaultAppearance(),
    states: {},
    folderTarget: null,
  };
}

export function createPage(name = 'Page 1'): Page {
  return {
    id: createId('page'),
    name,
    parentFolderId: null,
    slots: {
      keys: Array.from({ length: 8 }, (_, position) => createControlSlot('key', position)),
      dials: Array.from({ length: 4 }, (_, position) => createControlSlot('dial', position)),
      touch_regions: Array.from({ length: 4 }, (_, position) => createControlSlot('touch', position)),
    },
  };
}

export function createProfile(name = 'Default Profile', deviceId = 'stream-deck-plus'): Profile {
  const page = createPage();
  return {
    id: createId('profile'),
    name,
    device_id: deviceId,
    pages: [page],
    active_page_id: page.id,
    app_match: [],
  };
}

export function createDefaultWorkspace(): Workspace {
  const profile = createProfile();
  return {
    schema_version: 1,
    active_device_id: 'stream-deck-plus',
    active_profile_id: profile.id,
    profiles: [profile],
    assets: [],
    preferences: {
      actionPanelWidth: 260,
      inspectorHeight: 250,
      actionPanelCollapsed: false,
      inspectorCollapsed: false,
      zoom: 1,
    },
  };
}

export function cloneWorkspace(workspace: Workspace): Workspace {
  return structuredClone(workspace);
}

export function getActiveProfile(workspace: Workspace): Profile {
  const profile = workspace.profiles.find((candidate) => candidate.id === workspace.active_profile_id)
    ?? workspace.profiles.find((candidate) => candidate.device_id === workspace.active_device_id)
    ?? workspace.profiles[0];
  if (!profile) throw new Error('Workspace must contain at least one profile.');
  return profile;
}

export function getActivePage(workspace: Workspace): Page {
  const profile = getActiveProfile(workspace);
  const page = profile.pages.find((candidate) => candidate.id === profile.active_page_id) ?? profile.pages[0];
  if (!page) throw new Error('Profile must contain at least one page.');
  return page;
}

export function allSlots(page: Page): ControlSlot[] {
  return [...page.slots.keys, ...page.slots.dials, ...page.slots.touch_regions];
}

export function findSlot(page: Page, selection: ControlSelection): ControlSlot | undefined {
  const list = selection.kind === 'key'
    ? page.slots.keys
    : selection.kind === 'dial'
      ? page.slots.dials
      : page.slots.touch_regions;
  return list.find((slot) => slot.id === selection.slotId);
}
