import type { CSSProperties, DragEventHandler } from 'react';
import type { Appearance, ControlSlot } from '../model/workspace';

export interface DialControlProps {
  slot: ControlSlot;
  appearance: Appearance;
  selected: boolean;
  iconUrl?: string;
  backgroundUrl?: string;
  onSelect: () => void;
  onDragStart?: DragEventHandler<HTMLButtonElement>;
  onDragOver?: DragEventHandler<HTMLButtonElement>;
  onDrop?: DragEventHandler<HTMLButtonElement>;
}

function backgroundSize(value: ControlSlot['appearance']['fitMode']): CSSProperties['backgroundSize'] {
  return value === 'stretch' ? '100% 100%' : value;
}

export function DialControl({ slot, appearance, selected, iconUrl, backgroundUrl, onSelect, onDragStart, onDragOver, onDrop }: DialControlProps) {
  const labelStyle: CSSProperties = {
    color: appearance.textColor,
    fontFamily: appearance.fontFamily,
    fontSize: `${Math.max(8, Math.min(18, appearance.fontSize - 3))}px`,
    fontWeight: appearance.fontWeight,
    transform: `translate(${appearance.titleOffsetX}px, ${appearance.titleOffsetY}px)`,
  };
  const knobStyle: CSSProperties = {
    backgroundColor: appearance.backgroundColor,
    backgroundImage: backgroundUrl ? `url(${JSON.stringify(backgroundUrl)})` : undefined,
    backgroundPosition: 'center',
    backgroundRepeat: 'no-repeat',
    backgroundSize: backgroundSize(appearance.fitMode),
  };
  return <button draggable data-testid="dial" className={`dial-control${selected ? ' selected' : ''}`} onClick={onSelect} onDragStart={onDragStart} onDragOver={onDragOver} onDrop={onDrop}>
    <span className="dial-knob" style={knobStyle}>{iconUrl ? <img src={iconUrl} alt="" style={{ opacity: appearance.iconOpacity }} /> : appearance.iconAssetId ? <span className="dial-art" style={{ opacity: appearance.iconOpacity }}>◆</span> : null}</span>
    {appearance.titleVisible && <span className="dial-label" style={labelStyle}>{appearance.title || `Dial ${slot.position + 1}`}</span>}
  </button>;
}
