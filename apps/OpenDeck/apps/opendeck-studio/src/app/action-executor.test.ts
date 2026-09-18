import { describe, expect, it } from 'vitest';
import { resolveActionExecution } from './action-executor';

const context = {
  pages: [{ id: 'p1', name: 'One' }, { id: 'p2', name: 'Two' }, { id: 'p3', name: 'Three' }],
  currentPageId: 'p2',
  profiles: [{ id: 'a', name: 'A' }, { id: 'b', name: 'B' }],
  currentProfileId: 'a',
};

describe('action execution resolver', () => {
  it('resolves navigation actions without invoking services', () => {
    expect(resolveActionExecution({ definitionId: 'editor.nextPage', config: {} }, context)).toEqual({ kind: 'editor.setPage', pageId: 'p3' });
    expect(resolveActionExecution({ definitionId: 'editor.previousPage', config: {} }, context)).toEqual({ kind: 'editor.setPage', pageId: 'p1' });
    expect(resolveActionExecution({ definitionId: 'editor.folder', config: { pageId: 'p1' } }, context)).toEqual({ kind: 'editor.setPage', pageId: 'p1' });
    expect(resolveActionExecution({ definitionId: 'editor.switchProfile', config: { profileId: 'b' } }, context)).toEqual({ kind: 'editor.setProfile', profileId: 'b' });
  });

  it('marks stream and record toggles as confirmation-required', () => {
    expect(resolveActionExecution({ definitionId: 'obs.toggleStream', config: {} }, context)).toEqual({ kind: 'obs.toggleStream', requiresConfirmation: true });
    expect(resolveActionExecution({ definitionId: 'obs.toggleRecord', config: {} }, context)).toEqual({ kind: 'obs.toggleRecord', requiresConfirmation: true });
  });

  it('resolves the OBS launch/close toggle as a first-class action', () => {
    expect(resolveActionExecution({ definitionId: 'obs.toggleApp', config: {} }, context)).toEqual({ kind: 'obs.toggleApp' });
  });

  it('rejects incomplete configuration', () => {
    expect(resolveActionExecution({ definitionId: 'obs.scene', config: { sceneName: '' } }, context).kind).toBe('invalid');
    expect(resolveActionExecution({ definitionId: 'obs.toggleMute', config: { inputName: '' } }, context).kind).toBe('invalid');
    expect(resolveActionExecution({ definitionId: 'editor.folder', config: { pageId: 'missing' } }, context).kind).toBe('invalid');
  });
});
