import type { ControlSlot, DialStackEntry } from './workspace';

export function activeDialStackEntry(slot: ControlSlot): DialStackEntry | undefined {
  const stack = slot.dialStack;
  if (!stack || stack.entries.length === 0) return undefined;
  const index = Math.min(Math.max(0, stack.activeIndex), stack.entries.length - 1);
  return stack.entries[index];
}

export function effectiveDialBindings(slot: ControlSlot): ControlSlot['bindings'] {
  if (slot.kind !== 'dial') return slot.bindings;
  return activeDialStackEntry(slot)?.bindings ?? slot.bindings;
}

export function dialStackStatus(slot: ControlSlot): { label: string; index: number; total: number } | null {
  const stack = slot.dialStack;
  const entry = activeDialStackEntry(slot);
  if (!stack || !entry) return null;
  const index = Math.min(Math.max(0, stack.activeIndex), stack.entries.length - 1);
  return { label: entry.label || `Stack ${index + 1}`, index: index + 1, total: stack.entries.length };
}
