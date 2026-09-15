import { describe, expect, it } from 'vitest';
import { createDefaultWorkspace, getActivePage, getActiveProfile } from './workspace';

describe('workspace model', () => {
  it('creates one profile/page with 8 keys, 4 dials, 4 touch regions', () => {
    const ws = createDefaultWorkspace();
    expect(ws.schema_version).toBe(1);
    expect(ws.profiles).toHaveLength(1);
    expect(getActiveProfile(ws).pages).toHaveLength(1);
    const page = getActivePage(ws);
    expect(page.slots.keys).toHaveLength(8);
    expect(page.slots.dials).toHaveLength(4);
    expect(page.slots.touch_regions).toHaveLength(4);
  });

  it('uses stable ids instead of position as identity', () => {
    const page = getActivePage(createDefaultWorkspace());
    expect(new Set(page.slots.keys.map((slot) => slot.id)).size).toBe(8);
  });
});
