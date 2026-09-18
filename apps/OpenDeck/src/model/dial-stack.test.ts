import { describe, expect, it } from 'vitest';
import { createPage } from './workspace';
import { activeDialStackEntry, dialStackStatus, effectiveDialBindings } from './dial-stack';

describe('dial stack model', () => {
  it('uses base bindings when a dial has no stack', () => {
    const dial = createPage().slots.dials[0];
    dial.bindings.rotateRight = { definitionId: 'editor.nextPage', config: {} };
    expect(effectiveDialBindings(dial).rotateRight?.definitionId).toBe('editor.nextPage');
    expect(activeDialStackEntry(dial)).toBeUndefined();
    expect(dialStackStatus(dial)).toBeNull();
  });

  it('resolves the active stack entry and clamps stale indices', () => {
    const dial = createPage().slots.dials[0];
    dial.dialStack = {
      behavior: 'pressCycle',
      activeIndex: 7,
      entries: [
        { id: 'stack-a', label: 'OBS', bindings: { rotateRight: { definitionId: 'obs.scene', config: { sceneName: 'Game' } } } },
        { id: 'stack-b', label: 'Media', bindings: { rotateRight: { definitionId: 'editor.nextPage', config: {} } } },
      ],
    };
    expect(activeDialStackEntry(dial)?.id).toBe('stack-b');
    expect(effectiveDialBindings(dial).rotateRight?.definitionId).toBe('editor.nextPage');
    expect(dialStackStatus(dial)).toEqual({ label: 'Media', index: 2, total: 2 });
  });
});
