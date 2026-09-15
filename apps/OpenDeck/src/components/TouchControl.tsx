import type { ControlSlot } from '../model/workspace';

export interface TouchControlProps { slot: ControlSlot; selected: boolean; onSelect: () => void }

export function TouchControl({ slot, selected, onSelect }: TouchControlProps) {
  return <button data-testid="touch-control" className={`touch-control${selected ? ' selected' : ''}`} onClick={onSelect} style={{ backgroundColor: slot.appearance.backgroundColor, color: slot.appearance.textColor }}>
    {slot.appearance.title || `Touch ${slot.position + 1}`}
  </button>;
}
