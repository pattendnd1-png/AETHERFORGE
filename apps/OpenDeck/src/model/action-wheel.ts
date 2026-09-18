import type { ActionWheelEntry, ControlSlot } from './workspace';

export function activeActionWheelEntry(slot: ControlSlot): ActionWheelEntry | undefined {
  const wheel = slot.actionWheel;
  if (!wheel || wheel.entries.length === 0) return undefined;
  const index = Math.min(Math.max(0, wheel.activeIndex), wheel.entries.length - 1);
  return wheel.entries[index];
}

export function actionWheelStatus(slot: ControlSlot): { label: string; index: number; total: number } | null {
  const wheel = slot.actionWheel;
  const entry = activeActionWheelEntry(slot);
  if (!wheel || !entry) return null;
  const index = Math.min(Math.max(0, wheel.activeIndex), wheel.entries.length - 1);
  return { label: entry.label || `Action ${index + 1}`, index: index + 1, total: wheel.entries.length };
}

export function stepActionWheelIndex(activeIndex: number, total: number, direction: -1 | 1): number {
  if (total <= 0) return 0;
  const current = Math.min(Math.max(0, activeIndex), total - 1);
  return (current + direction + total) % total;
}
