import type { ControlSlot } from '../model/workspace';

export interface DialControlProps { slot: ControlSlot; selected: boolean; onSelect: () => void }

export function DialControl({ slot, selected, onSelect }: DialControlProps) {
  return <button data-testid="dial" className={`dial-control${selected ? ' selected' : ''}`} onClick={onSelect}>
    <span className="dial-knob" />
    <span className="dial-label" style={{ color: slot.appearance.textColor }}>{slot.appearance.title || `Dial ${slot.position + 1}`}</span>
  </button>;
}
