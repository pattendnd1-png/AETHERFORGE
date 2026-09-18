import { describe, expect, it } from 'vitest';
import { createQualificationWorkspace } from './demoWorkspace';

describe('OpenDeck+ 2.0.48 qualification workspace', () => {
  it('creates the canonical v2.0.48 visual qualification workspace', () => {
    const workspace = createQualificationWorkspace();
    const page = workspace.profiles[0].pages[0];
    expect(page.slots.keys.map((slot) => slot.appearance.title)).toEqual([
      'Scene', 'Mic', 'Spotify', 'Browser', 'OBS', 'Twitch', 'Discord', 'System',
    ]);
    expect(page.slots.touch_regions.map((slot) => slot.appearance.title)).toEqual([
      'Volume', 'Zoom', 'Brightness', 'Media',
    ]);
    expect(page.slots.dials.map((slot) => slot.appearance.title)).toEqual([
      'Volume', 'Zoom', 'Brightness', 'Media',
    ]);
    expect(page.slots.dials[0].dialStack?.entries.map((entry) => entry.label)).toEqual([
      'Volume', 'OBS Studio', 'Spotify',
    ]);
    expect(page.slots.dials[0].dialStack?.activeIndex).toBe(0);
    expect(page.slots.dials[1].actionWheel?.entries.map((entry) => entry.label)).toEqual([
      'Stream', 'Record', 'Marketplace',
    ]);
    expect(page.slots.dials[1].actionWheel?.activeIndex).toBe(0);
  });
});
