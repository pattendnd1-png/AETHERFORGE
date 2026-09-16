import type { ControlSelection, Interaction, Page } from '../model/workspace';
import type { StreamDeckInputEvent } from '../types';

export interface HardwareExecution {
  selection: ControlSelection;
  interaction: Interaction;
}

const MAX_ROTATION_ACTIONS = 16;

function execution(kind: ControlSelection['kind'], slotId: string, interaction: Interaction): HardwareExecution {
  return { selection: { kind, slotId }, interaction };
}

export function resolveHardwareExecutions(event: StreamDeckInputEvent, page: Page): HardwareExecution[] {
  switch (event.kind) {
    case 'keyDown': {
      const slot = page.slots.keys[event.index];
      return slot ? [execution('key', slot.id, 'press')] : [];
    }
    case 'keyUp':
      return [];
    case 'dialDown': {
      const slot = page.slots.dials[event.index];
      return slot ? [execution('dial', slot.id, 'press')] : [];
    }
    case 'dialUp':
      return [];
    case 'dialRotate': {
      const slot = page.slots.dials[event.index];
      if (!slot || event.ticks === 0) return [];
      const interaction: Interaction = event.ticks < 0
        ? (event.pressed ? 'pressRotateLeft' : 'rotateLeft')
        : (event.pressed ? 'pressRotateRight' : 'rotateRight');
      const count = Math.min(MAX_ROTATION_ACTIONS, Math.abs(event.ticks));
      return Array.from({ length: count }, () => execution('dial', slot.id, interaction));
    }
    case 'touchTap': {
      const slot = page.slots.touch_regions[event.region];
      return slot ? [execution('touch', slot.id, 'touch')] : [];
    }
    case 'touchPress': {
      const slot = page.slots.touch_regions[event.region];
      if (!slot) return [];
      const interaction: Interaction = slot.bindings.longPress ? 'longPress' : 'touch';
      return [execution('touch', slot.id, interaction)];
    }
    case 'touchFlick': {
      const slot = page.slots.touch_regions[event.region];
      return slot ? [execution('touch', slot.id, 'touch')] : [];
    }
  }
}
