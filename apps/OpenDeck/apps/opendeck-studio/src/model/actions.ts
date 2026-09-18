import type { ActionInstance, ControlKind, Interaction } from './workspace';
import { createPluginActionInstance, externalActionDefinition, isPluginDefinitionId } from './plugin-actions';

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
  { id: 'editor.multiAction', label: 'Multi Action', group: 'Multi Actions', supportedKinds: ['key', 'dial', 'touch'], defaultInteraction: 'press', inspector: [], defaultConfig: { steps: [] } },
  { id: 'editor.keyLogic', label: 'Key Logic', group: 'Multi Actions', supportedKinds: ['key'], defaultInteraction: 'press', inspector: [], defaultConfig: { press: null, doublePress: null, longPress: null, holdMs: 500, doubleMs: 300 } },
  { id: 'editor.dialStack', label: 'Dial Stack', group: 'Dial Stacks', supportedKinds: ['dial'], defaultInteraction: 'press', inspector: [], defaultConfig: {} },
  { id: 'editor.actionWheel', label: 'Action Wheel', group: 'Action Wheels', supportedKinds: ['dial'], defaultInteraction: 'press', inspector: [], defaultConfig: {} },
];

export function getActionDefinition(id: string): ActionDefinition {
  const definition = ACTION_DEFINITIONS.find((candidate) => candidate.id === id) ?? externalActionDefinition(id);
  if (definition) return definition;
  if (isPluginDefinitionId(id)) {
    return { id, label: 'Unavailable Plugin Action', group: 'Plugin unavailable', supportedKinds: ['key','dial','touch'], defaultInteraction: 'press', inspector: [], defaultConfig: {} };
  }
  throw new Error(`Unknown action definition: ${id}`);
}

export function createActionInstance(id: string): ActionInstance {
  if (isPluginDefinitionId(id)) return createPluginActionInstance(id);
  const definition = getActionDefinition(id);
  return { definitionId: definition.id, config: structuredClone(definition.defaultConfig) };
}

export function supportsControl(id: string, kind: ControlKind): boolean {
  return getActionDefinition(id).supportedKinds.includes(kind);
}
