import type { ActionInstance, ControlKind, Interaction } from './workspace';

export interface ActionInspectorField {
  key: string;
  label: string;
  kind: 'text' | 'select';
  options?: string[];
}

export interface ActionDefinition {
  id: string;
  label: string;
  group: string;
  supportedKinds: ControlKind[];
  defaultInteraction: Interaction;
  inspector: ActionInspectorField[];
  defaultConfig: Record<string, unknown>;
}

export const ACTION_DEFINITIONS: ActionDefinition[] = [
  { id: 'obs.scene', label: 'Scene', group: 'OBS Studio', supportedKinds: ['key', 'dial', 'touch'], defaultInteraction: 'press', inspector: [{ key: 'sceneName', label: 'Scene', kind: 'text' }], defaultConfig: { sceneName: '' } },
  { id: 'obs.toggleStream', label: 'Toggle Stream', group: 'OBS Studio', supportedKinds: ['key', 'dial', 'touch'], defaultInteraction: 'press', inspector: [], defaultConfig: {} },
  { id: 'obs.toggleRecord', label: 'Toggle Record', group: 'OBS Studio', supportedKinds: ['key', 'dial', 'touch'], defaultInteraction: 'press', inspector: [], defaultConfig: {} },
  { id: 'obs.toggleMute', label: 'Toggle Input Mute', group: 'OBS Studio', supportedKinds: ['key', 'dial', 'touch'], defaultInteraction: 'press', inspector: [{ key: 'inputName', label: 'Input', kind: 'text' }], defaultConfig: { inputName: '' } },
  { id: 'marketplace.open', label: 'Open Marketplace', group: 'Elgato', supportedKinds: ['key', 'dial', 'touch'], defaultInteraction: 'press', inspector: [], defaultConfig: {} },
  { id: 'editor.folder', label: 'Folder', group: 'Navigation', supportedKinds: ['key', 'touch'], defaultInteraction: 'press', inspector: [{ key: 'pageId', label: 'Page', kind: 'select', options: [] }], defaultConfig: { pageId: '' } },
  { id: 'editor.nextPage', label: 'Next Page', group: 'Navigation', supportedKinds: ['key', 'dial', 'touch'], defaultInteraction: 'press', inspector: [], defaultConfig: {} },
  { id: 'editor.previousPage', label: 'Previous Page', group: 'Navigation', supportedKinds: ['key', 'dial', 'touch'], defaultInteraction: 'press', inspector: [], defaultConfig: {} },
  { id: 'editor.switchProfile', label: 'Switch Profile', group: 'Navigation', supportedKinds: ['key', 'dial', 'touch'], defaultInteraction: 'press', inspector: [{ key: 'profileId', label: 'Profile', kind: 'select', options: [] }], defaultConfig: { profileId: '' } },
  { id: 'editor.blank', label: 'Blank', group: 'Navigation', supportedKinds: ['key', 'dial', 'touch'], defaultInteraction: 'press', inspector: [], defaultConfig: {} },
];

export function getActionDefinition(id: string): ActionDefinition {
  const definition = ACTION_DEFINITIONS.find((candidate) => candidate.id === id);
  if (!definition) throw new Error(`Unknown action definition: ${id}`);
  return definition;
}

export function createActionInstance(id: string): ActionInstance {
  const definition = getActionDefinition(id);
  return { definitionId: definition.id, config: structuredClone(definition.defaultConfig) };
}

export function supportsControl(id: string, kind: ControlKind): boolean {
  return getActionDefinition(id).supportedKinds.includes(kind);
}
