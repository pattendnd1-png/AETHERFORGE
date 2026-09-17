import { describe, expect, it } from 'vitest';
import { createQualificationWorkspace } from './demoWorkspace';

describe('OpenDeck+ 2.0.26 qualification workspace', () => {
  it('creates the canonical v2.0.26 visual qualification workspace', () => {
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
  });
});
