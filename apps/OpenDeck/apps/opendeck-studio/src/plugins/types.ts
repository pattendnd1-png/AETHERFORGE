import type { ControlKind, Interaction } from '../model/workspace';

export type PluginRuntimeKind = 'node' | 'native' | 'wine' | 'opendeck';
export type PluginCompatibility = 'native' | 'wine' | 'node' | 'drm-protected' | 'unsupported';
export type PluginProcessState = 'inactive' | 'starting' | 'active' | 'stopped' | 'crashed' | 'error';

export interface PluginActionDescriptor {
  id: string;
  pluginUuid: string;
  uuid: string;
  name: string;
  category: string;
  controllers: Array<'Keypad' | 'Encoder' | 'Neo'>;
  supportedKinds: ControlKind[];
  defaultInteraction: Interaction;
  propertyInspectorPath: string | null;
  states: Array<{ name?: string; image?: string }>;
  supportedInMultiActions: boolean;
  supportedInKeyLogicActions: boolean;
  disableAutomaticStates: boolean;
  visibleInActionsList: boolean;
}

export interface PluginDescriptor {
  uuid: string;
  name: string;
  version: string;
  author: string;
  description: string;
  sourceKind: 'elgato' | 'opendeck';
  root: string;
  enabled: boolean;
  active: boolean;
  processState: PluginProcessState;
  compatibility: PluginCompatibility;
  runtimeKind: PluginRuntimeKind | null;
  lastError: string | null;
  sdkVersion: number;
  minimumSoftwareVersion: string | null;
  propertyInspectorPath: string | null;
  actions: PluginActionDescriptor[];
  profiles: Array<{
    name: string;
    deviceType: number;
    readonly: boolean;
    autoInstall: boolean;
    dontAutoSwitchWhenInstalled: boolean;
  }>;
}

export interface PluginHostStatus {
  protocolVersion: string;
  streamDeckCompatibilityTarget: string;
  websocketHost: string;
  installed: number;
  active: number;
  enabled: number;
}

export interface PluginFeedback {
  context: string;
  pluginUuid?: string;
  actionUuid?: string;
  title?: string | null;
  imageAssetId?: string | null;
  imagePath?: string | null;
  imageDataUrl?: string | null;
  state?: number | null;
  feedback?: Record<string, unknown> | null;
  feedbackLayout?: string | null;
  triggerDescription?: Record<string, string> | null;
  signal?: 'ok' | 'alert' | null;
}

export interface PluginDispatchRequest {
  pluginUuid: string;
  actionUuid: string;
  context: string;
  device: string;
  controlKind: ControlKind;
  position: number;
  interaction?: Interaction;
  hardwareEvent?: unknown;
  isInMultiAction?: boolean;
  userDesiredState?: number | null;
}

export interface PluginPropertyInspectorSession {
  pluginUuid: string;
  actionUuid: string;
  context: string;
  url: string;
}
