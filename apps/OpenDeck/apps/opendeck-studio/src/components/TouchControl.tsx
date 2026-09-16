import type { CSSProperties, DragEventHandler } from 'react';
import type { Appearance, ControlSlot } from '../model/workspace';

export interface TouchControlProps {
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

export function TouchControl({ slot, appearance, selected, iconUrl, backgroundUrl, onSelect, onDragStart, onDragOver, onDrop }: TouchControlProps) {
  const style: CSSProperties = {
    backgroundColor: appearance.backgroundColor,
    backgroundImage: backgroundUrl ? `url(${JSON.stringify(backgroundUrl)})` : undefined,
    backgroundPosition: 'center',
    backgroundRepeat: 'no-repeat',
    backgroundSize: backgroundSize(appearance.fitMode),
    color: appearance.textColor,
    fontFamily: appearance.fontFamily,
    fontSize: `${Math.max(8, Math.min(18, appearance.fontSize - 2))}px`,
    fontWeight: appearance.fontWeight,
    textAlign: appearance.horizontalAlign,
  };
  const titleStyle: CSSProperties = {
    transform: `translate(${appearance.titleOffsetX}px, ${appearance.titleOffsetY}px)`,
  };
  return <button draggable data-testid="touch-control" className={`touch-control${selected ? ' selected' : ''}`} onClick={onSelect} onDragStart={onDragStart} onDragOver={onDragOver} onDrop={onDrop} style={style}>
    {iconUrl ? <img className="touch-art-image" src={iconUrl} alt="" style={{ opacity: appearance.iconOpacity }} /> : appearance.iconAssetId ? <span className="touch-art" style={{ opacity: appearance.iconOpacity }}>◆</span> : null}
    {appearance.titleVisible && <span className="touch-title" style={titleStyle}>{appearance.title || `Touch ${slot.position + 1}`}</span>}
  </button>;
}
