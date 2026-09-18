import type { ActionDefinition } from './actions';
import type { ActionInstance, ControlKind, Interaction } from './workspace';
import type { PluginActionDescriptor, PluginDescriptor } from '../plugins/types';

const PREFIX = 'plugin:';
let externalDefinitions = new Map<string, ActionDefinition>();
let externalDescriptors = new Map<string, PluginActionDescriptor>();

export function pluginDefinitionId(pluginUuid: string, actionUuid: string): string {
  return `${PREFIX}${pluginUuid}:${actionUuid}`;
}

export function isPluginDefinitionId(id: string): boolean {
  return id.startsWith(PREFIX);
}

export function parsePluginDefinitionId(id: string): { pluginUuid: string; actionUuid: string } | null {
  if (!isPluginDefinitionId(id)) return null;
  const rest = id.slice(PREFIX.length);
  const separator = rest.indexOf(':');
  if (separator < 1 || separator === rest.length - 1) return null;
  return { pluginUuid: rest.slice(0, separator), actionUuid: rest.slice(separator + 1) };
}

function kindsForControllers(controllers: PluginActionDescriptor['controllers']): ControlKind[] {
  const kinds = new Set<ControlKind>();
  for (const controller of controllers) {
    if (controller === 'Keypad' || controller === 'Neo') kinds.add('key');
    if (controller === 'Encoder') {
      kinds.add('dial');
      kinds.add('touch');
    }
  }
  return [...kinds];
}

function defaultInteraction(kinds: ControlKind[]): Interaction {
  if (kinds.includes('key')) return 'press';
  if (kinds.includes('dial')) return 'press';
  return 'touch';
}

export function replacePluginActionDefinitions(plugins: PluginDescriptor[]): ActionDefinition[] {
  const nextDefinitions = new Map<string, ActionDefinition>();
  const nextDescriptors = new Map<string, PluginActionDescriptor>();
  const enabledIds = new Set<string>();
  for (const plugin of plugins) {
    for (const action of plugin.actions) {
      const id = pluginDefinitionId(plugin.uuid, action.uuid);
      const kinds = action.supportedKinds.length ? action.supportedKinds : kindsForControllers(action.controllers);
      nextDefinitions.set(id, {
        id,
        label: action.name,
        group: plugin.name,
        supportedKinds: kinds,
        defaultInteraction: action.defaultInteraction ?? defaultInteraction(kinds),
        inspector: [],
        defaultConfig: {
          pluginUuid: plugin.uuid,
          actionUuid: action.uuid,
          pluginName: plugin.name,
          propertyInspectorPath: action.propertyInspectorPath,
          settings: {},
          resources: {},
          state: 0,
        },
      });
      nextDescriptors.set(id, action);
      if (plugin.enabled && action.visibleInActionsList !== false) enabledIds.add(id);
    }
  }
  externalDefinitions = nextDefinitions;
  externalDescriptors = nextDescriptors;
  return [...externalDefinitions.values()].filter((definition) => enabledIds.has(definition.id));
}

export function externalActionDefinitions(): ActionDefinition[] {
  return [...externalDefinitions.values()];
}

export function externalActionDefinition(id: string): ActionDefinition | undefined {
  return externalDefinitions.get(id);
}

export function externalActionDescriptor(id: string): PluginActionDescriptor | undefined {
  return externalDescriptors.get(id);
}

export function createPluginActionInstance(id: string): ActionInstance {
  const definition = externalDefinitions.get(id);
  if (!definition) throw new Error(`Unknown plugin action definition: ${id}`);
  const context = typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function'
    ? `plugin-context-${crypto.randomUUID()}`
    : `plugin-context-${Date.now()}-${Math.random().toString(16).slice(2)}`;
  return {
    definitionId: definition.id,
    config: {
      ...structuredClone(definition.defaultConfig),
      context,
    },
  };
}

export function pluginActionAllowedInMultiAction(id: string): boolean {
  return externalDescriptors.get(id)?.supportedInMultiActions !== false;
}

export function pluginActionAllowedInKeyLogic(id: string): boolean {
  return externalDescriptors.get(id)?.supportedInKeyLogicActions !== false;
}
