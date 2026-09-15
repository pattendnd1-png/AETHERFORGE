import { createActionInstance } from './actions';
import { cloneWorkspace, getActivePage, type Workspace } from './workspace';

interface LegacyAction {
  kind?: string;
  label?: string;
  sceneName?: string;
  inputName?: string;
}

const LEGACY_IDS = new Set([
  'obs.scene',
  'obs.toggleStream',
  'obs.toggleRecord',
  'obs.toggleMute',
  'marketplace.open',
]);

export function migrateLegacyKeys(value: unknown, workspace: Workspace): { workspace: Workspace; migrated: boolean } {
  const next = cloneWorkspace(workspace);
  if (!Array.isArray(value) || value.length !== 8) return { workspace: next, migrated: false };

  const page = getActivePage(next);
  value.forEach((raw, index) => {
    if (!raw || typeof raw !== 'object') return;
    const legacy = raw as LegacyAction;
    if (!legacy.kind || !LEGACY_IDS.has(legacy.kind)) return;
    const action = createActionInstance(legacy.kind);
    if (legacy.kind === 'obs.scene' && typeof legacy.sceneName === 'string') action.config.sceneName = legacy.sceneName;
    if (legacy.kind === 'obs.toggleMute' && typeof legacy.inputName === 'string') action.config.inputName = legacy.inputName;
    page.slots.keys[index].bindings.press = action;
    if (typeof legacy.label === 'string') page.slots.keys[index].appearance.title = legacy.label;
  });
  return { workspace: next, migrated: true };
}
