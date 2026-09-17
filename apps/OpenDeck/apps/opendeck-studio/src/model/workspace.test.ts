import { describe, expect, it } from 'vitest';
import { createDefaultWorkspace, getActivePage, getActiveProfile, resolveAppearance, resolveTouchStripPresentation } from './workspace';

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
  it('resolves active-state overrides over the base appearance', () => {
    const slot = getActivePage(createDefaultWorkspace()).slots.keys[0];
    slot.appearance.title = 'Default';
    slot.appearance.backgroundColor = '#111111';
    slot.states.active = { title: 'Live', backgroundColor: '#aa0000' };
    expect(resolveAppearance(slot, 'default').title).toBe('Default');
    expect(resolveAppearance(slot, 'active').title).toBe('Live');
    expect(resolveAppearance(slot, 'active').backgroundColor).toBe('#aa0000');
  });

});

describe('adaptive unified touch strip', () => {
  it('defaults to segmented and owns a durable unified slot', () => {
    const page = getActivePage(createDefaultWorkspace());
    expect(page.touchStrip.mode).toBe('segmented');
    expect(page.touchStrip.adaptiveFallback).toBe('segmented');
    expect(page.touchStrip.unifiedSlot.kind).toBe('touch');
    expect(page.touchStrip.unifiedSlot.id).not.toBe(page.slots.touch_regions[0].id);
  });

  it('resolves explicit and application-adaptive presentations', () => {
    const page = getActivePage(createDefaultWorkspace());
    page.touchStrip.mode = 'unified';
    expect(resolveTouchStripPresentation(page, 'org.spotify.Client')).toBe('unified');
    page.touchStrip.mode = 'adaptive';
    page.touchStrip.adaptiveFallback = 'segmented';
    page.touchStrip.adaptiveRules = [
      { pattern: 'spotify', presentation: 'unified' },
      { pattern: 'obs', presentation: 'segmented' },
    ];
    expect(resolveTouchStripPresentation(page, 'com.spotify.Client')).toBe('unified');
    expect(resolveTouchStripPresentation(page, 'com.obsproject.Studio')).toBe('segmented');
    expect(resolveTouchStripPresentation(page, 'org.mozilla.firefox')).toBe('segmented');
  });
});
