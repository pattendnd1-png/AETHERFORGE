import type { ControlSelection, Interaction, Page, TouchStripPresentation } from '../model/workspace';
import type { StreamDeckInputEvent } from '../types';

export interface TouchExecutionContext {
  x?: number;
  y?: number;
  normalizedX?: number;
  normalizedY?: number;
  startX?: number;
  startY?: number;
  endX?: number;
  endY?: number;
  normalizedStartX?: number;
  normalizedEndX?: number;
}

export interface HardwareExecution {
  selection: ControlSelection;
  interaction: Interaction;
  touch?: TouchExecutionContext;
}

const MAX_ROTATION_ACTIONS = 16;
const TOUCH_MAX_X = 799;
const TOUCH_MAX_Y = 99;

function execution(kind: ControlSelection['kind'], slotId: string, interaction: Interaction, touch?: TouchExecutionContext): HardwareExecution {
  return touch ? { selection: { kind, slotId }, interaction, touch } : { selection: { kind, slotId }, interaction };
}

function unit(value: number, max: number): number {
  return Math.min(1, Math.max(0, value / max));
}

function unifiedTouchContext(event: Extract<StreamDeckInputEvent, { kind: 'touchTap' | 'touchPress' | 'touchFlick' }>): TouchExecutionContext {
  if (event.kind === 'touchFlick') {
    return {
      startX: event.startX,
      startY: event.startY,
      endX: event.endX,
      endY: event.endY,
      normalizedStartX: unit(event.startX, TOUCH_MAX_X),
      normalizedEndX: unit(event.endX, TOUCH_MAX_X),
    };
  }
  return {
    x: event.x,
    y: event.y,
    normalizedX: unit(event.x, TOUCH_MAX_X),
    normalizedY: unit(event.y, TOUCH_MAX_Y),
  };
}

export function resolveHardwareExecutions(event: StreamDeckInputEvent, page: Page, presentation: TouchStripPresentation = 'segmented'): HardwareExecution[] {
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
      const wheelDial = page.slots.dials[event.region];
      if (wheelDial?.actionWheel) return [execution('dial', wheelDial.id, 'press')];
      if (presentation === 'unified') {
        return [execution('touch', page.touchStrip.unifiedSlot.id, 'touch', unifiedTouchContext(event))];
      }
      const slot = page.slots.touch_regions[event.region];
      return slot ? [execution('touch', slot.id, 'touch')] : [];
    }
    case 'touchPress': {
      const slot = presentation === 'unified' ? page.touchStrip.unifiedSlot : page.slots.touch_regions[event.region];
      if (!slot) return [];
      const interaction: Interaction = slot.bindings.longPress ? 'longPress' : 'touch';
      return [execution('touch', slot.id, interaction, presentation === 'unified' ? unifiedTouchContext(event) : undefined)];
    }
    case 'touchFlick': {
      if (presentation === 'unified') {
        return [execution('touch', page.touchStrip.unifiedSlot.id, 'touch', unifiedTouchContext(event))];
      }
      const slot = page.slots.touch_regions[event.region];
      return slot ? [execution('touch', slot.id, 'touch')] : [];
    }
  }
}
