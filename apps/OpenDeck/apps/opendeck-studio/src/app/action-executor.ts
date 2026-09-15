export interface ExecutableAction {
  definitionId: string;
  config: Record<string, unknown>;
}

export interface ActionExecutionContext {
  pages: Array<{ id: string; name: string }>;
  currentPageId: string;
  profiles: Array<{ id: string; name: string }>;
  currentProfileId: string;
}

export type ResolvedActionExecution =
  | { kind: 'obs.scene'; sceneName: string }
  | { kind: 'obs.toggleStream'; requiresConfirmation: true }
  | { kind: 'obs.toggleRecord'; requiresConfirmation: true }
  | { kind: 'obs.toggleMute'; inputName: string }
  | { kind: 'marketplace.open'; url: string }
  | { kind: 'editor.setPage'; pageId: string }
  | { kind: 'editor.setProfile'; profileId: string }
  | { kind: 'noop'; message: string }
  | { kind: 'invalid'; message: string };

function configuredString(config: Record<string, unknown>, key: string): string {
  const value = config[key];
  return typeof value === 'string' ? value.trim() : '';
}

export function resolveActionExecution(action: ExecutableAction, context: ActionExecutionContext): ResolvedActionExecution {
  switch (action.definitionId) {
    case 'obs.scene': {
      const sceneName = configuredString(action.config, 'sceneName');
      return sceneName ? { kind: 'obs.scene', sceneName } : { kind: 'invalid', message: 'Choose an OBS scene first.' };
    }
    case 'obs.toggleStream':
      return { kind: 'obs.toggleStream', requiresConfirmation: true };
    case 'obs.toggleRecord':
      return { kind: 'obs.toggleRecord', requiresConfirmation: true };
    case 'obs.toggleMute': {
      const inputName = configuredString(action.config, 'inputName');
      return inputName ? { kind: 'obs.toggleMute', inputName } : { kind: 'invalid', message: 'Choose an OBS input first.' };
    }
    case 'marketplace.open':
      return { kind: 'marketplace.open', url: 'https://marketplace.elgato.com' };
    case 'editor.folder': {
      const pageId = configuredString(action.config, 'pageId');
      return context.pages.some((page) => page.id === pageId)
        ? { kind: 'editor.setPage', pageId }
        : { kind: 'invalid', message: 'Choose a valid folder page first.' };
    }
    case 'editor.nextPage': {
      const index = context.pages.findIndex((page) => page.id === context.currentPageId);
      if (index < 0 || index >= context.pages.length - 1) return { kind: 'noop', message: 'Already on the last page.' };
      return { kind: 'editor.setPage', pageId: context.pages[index + 1].id };
    }
    case 'editor.previousPage': {
      const index = context.pages.findIndex((page) => page.id === context.currentPageId);
      if (index <= 0) return { kind: 'noop', message: 'Already on the first page.' };
      return { kind: 'editor.setPage', pageId: context.pages[index - 1].id };
    }
    case 'editor.switchProfile': {
      const profileId = configuredString(action.config, 'profileId');
      return context.profiles.some((profile) => profile.id === profileId)
        ? { kind: 'editor.setProfile', profileId }
        : { kind: 'invalid', message: 'Choose a valid profile first.' };
    }
    case 'editor.blank':
      return { kind: 'noop', message: 'Blank action.' };
    default:
      return { kind: 'invalid', message: `Unsupported action: ${action.definitionId}` };
  }
}
