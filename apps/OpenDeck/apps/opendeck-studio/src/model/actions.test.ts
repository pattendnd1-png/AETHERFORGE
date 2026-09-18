import { describe, expect, it } from 'vitest';
import { getActionDefinition } from './actions';

describe('action registry', () => {
  it('keeps service and editor-native actions in one registry', () => {
    expect(getActionDefinition('obs.toggleStream').supportedKinds).toContain('key');
    expect(getActionDefinition('obs.toggleApp').label).toBe('Launch / Close OBS');
    expect(getActionDefinition('editor.folder').supportedKinds).toContain('touch');
  });
});
