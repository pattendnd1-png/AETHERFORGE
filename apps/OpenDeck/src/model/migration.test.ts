import { describe, expect, it } from 'vitest';
import { createDefaultWorkspace, getActivePage } from './workspace';
import { migrateLegacyKeys } from './migration';

describe('legacy key migration', () => {
  it('migrates exactly eight legacy keys into the default page', () => {
    const ws = createDefaultWorkspace();
    const legacy = Array.from({ length: 8 }, (_, i) => i === 0 ? { kind: 'obs.toggleStream', label: 'Go Live' } : null);
    const migrated = migrateLegacyKeys(legacy, ws);
    expect(migrated.migrated).toBe(true);
    expect(getActivePage(migrated.workspace).slots.keys[0].bindings.press?.definitionId).toBe('obs.toggleStream');
    expect(getActivePage(migrated.workspace).slots.keys[0].appearance.title).toBe('Go Live');
  });

  it('ignores invalid legacy shapes', () => {
    const ws = createDefaultWorkspace();
    const migrated = migrateLegacyKeys([{ kind: 'obs.toggleStream' }], ws);
    expect(migrated.migrated).toBe(false);
    expect(getActivePage(migrated.workspace).slots.keys[0].bindings.press).toBeUndefined();
  });
});
