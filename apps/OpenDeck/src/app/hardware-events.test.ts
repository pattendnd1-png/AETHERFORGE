import { describe, expect, it } from 'vitest';
import { createPage } from '../model/workspace';
import { resolveHardwareExecutions } from './hardware-events';

describe('hardware input resolver', () => {
  it('maps key down and ignores key up', () => {
    const page = createPage();
    expect(resolveHardwareExecutions({ kind: 'keyDown', index: 0 }, page)).toEqual([
      { selection: { kind: 'key', slotId: page.slots.keys[0].id }, interaction: 'press' },
    ]);
    expect(resolveHardwareExecutions({ kind: 'keyUp', index: 0 }, page)).toEqual([]);
  });

  it('expands signed dial ticks and clamps pathological reports', () => {
    const page = createPage();
    expect(resolveHardwareExecutions({ kind: 'dialRotate', index: 1, ticks: -3, pressed: false }, page)).toEqual(
      Array.from({ length: 3 }, () => ({
        selection: { kind: 'dial', slotId: page.slots.dials[1].id },
        interaction: 'rotateLeft',
      })),
    );
    expect(resolveHardwareExecutions({ kind: 'dialRotate', index: 1, ticks: 127, pressed: false }, page)).toHaveLength(16);
  });

  it('maps dial press and ignores dial release', () => {
    const page = createPage();
    expect(resolveHardwareExecutions({ kind: 'dialDown', index: 2 }, page)).toEqual([
      { selection: { kind: 'dial', slotId: page.slots.dials[2].id }, interaction: 'press' },
    ]);
    expect(resolveHardwareExecutions({ kind: 'dialUp', index: 2 }, page)).toEqual([]);
  });

  it('distinguishes pressed dial rotation from ordinary rotation', () => {
    const page = createPage();
    expect(resolveHardwareExecutions({ kind: 'dialRotate', index: 0, ticks: -2, pressed: true }, page)).toEqual([
      { selection: { kind: 'dial', slotId: page.slots.dials[0].id }, interaction: 'pressRotateLeft' },
      { selection: { kind: 'dial', slotId: page.slots.dials[0].id }, interaction: 'pressRotateLeft' },
    ]);
    expect(resolveHardwareExecutions({ kind: 'dialRotate', index: 0, ticks: 2, pressed: true }, page)).toEqual([
      { selection: { kind: 'dial', slotId: page.slots.dials[0].id }, interaction: 'pressRotateRight' },
      { selection: { kind: 'dial', slotId: page.slots.dials[0].id }, interaction: 'pressRotateRight' },
    ]);
  });

  it('maps touch x=650 to region 3 and prefers long press when configured', () => {
    const page = createPage();
    page.slots.touch_regions[3].bindings.longPress = { definitionId: 'editor.blank', config: {} };
    expect(resolveHardwareExecutions({ kind: 'touchTap', x: 650, y: 40, region: 3 }, page)).toEqual([
      { selection: { kind: 'touch', slotId: page.slots.touch_regions[3].id }, interaction: 'touch' },
    ]);
    expect(resolveHardwareExecutions({ kind: 'touchPress', x: 650, y: 40, region: 3 }, page)).toEqual([
      { selection: { kind: 'touch', slotId: page.slots.touch_regions[3].id }, interaction: 'longPress' },
    ]);
  });

  it('ignores out-of-range indices and uses the start region for flicks', () => {
    const page = createPage();
    expect(resolveHardwareExecutions({ kind: 'keyDown', index: 99 }, page)).toEqual([]);
    expect(resolveHardwareExecutions({ kind: 'touchFlick', startX: 10, startY: 20, endX: 700, endY: 20, region: 0 }, page)).toEqual([
      { selection: { kind: 'touch', slotId: page.slots.touch_regions[0].id }, interaction: 'touch' },
    ]);
  });
});
