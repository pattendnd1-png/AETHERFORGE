import type { CSSProperties } from 'react';
import type { ControlSlot } from '../model/workspace';

export interface KeyControlProps {
  slot: ControlSlot;
  selected: boolean;
  onSelect: () => void;
}

export function KeyControl({ slot, selected, onSelect }: KeyControlProps) {
  const style: CSSProperties = {
    backgroundColor: slot.appearance.backgroundColor,
    color: slot.appearance.textColor,
    opacity: slot.appearance.iconOpacity,
    fontFamily: slot.appearance.fontFamily,
    fontSize: `${slot.appearance.fontSize}px`,
    fontWeight: slot.appearance.fontWeight,
    textAlign: slot.appearance.horizontalAlign,
  };
  return <button data-testid="deck-key" className={`control key-control${selected ? ' selected' : ''}`} style={style} onClick={onSelect}>
    <span className="control-art" aria-hidden="true">{slot.appearance.iconAssetId ? '◆' : ''}</span>
    {slot.appearance.titleVisible && <span className="control-title">{slot.appearance.title || 'Key'}</span>}
    <small>{slot.position + 1}</small>
  </button>;
}
