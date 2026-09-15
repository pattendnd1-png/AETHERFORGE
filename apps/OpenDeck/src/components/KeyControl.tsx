import type { CSSProperties, DragEventHandler } from 'react';
import type { Appearance, ControlSlot } from '../model/workspace';

export interface KeyControlProps {
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

function verticalJustify(value: ControlSlot['appearance']['verticalAlign']): CSSProperties['justifyContent'] {
  if (value === 'top') return 'flex-start';
  if (value === 'middle') return 'center';
  return 'flex-end';
}

function objectFit(value: ControlSlot['appearance']['fitMode']): CSSProperties['objectFit'] {
  return value === 'stretch' ? 'fill' : value;
}

function backgroundSize(value: ControlSlot['appearance']['fitMode']): CSSProperties['backgroundSize'] {
  return value === 'stretch' ? '100% 100%' : value;
}

export function KeyControl({ slot, appearance, selected, iconUrl, backgroundUrl, onSelect, onDragStart, onDragOver, onDrop }: KeyControlProps) {
  const style: CSSProperties = {
    backgroundColor: appearance.backgroundColor,
    backgroundImage: backgroundUrl ? `url(${JSON.stringify(backgroundUrl)})` : undefined,
    backgroundPosition: 'center',
    backgroundRepeat: 'no-repeat',
    backgroundSize: backgroundSize(appearance.fitMode),
    color: appearance.textColor,
    fontFamily: appearance.fontFamily,
    fontSize: `${appearance.fontSize}px`,
    fontWeight: appearance.fontWeight,
    textAlign: appearance.horizontalAlign,
    justifyContent: verticalJustify(appearance.verticalAlign),
  };
  const titleStyle: CSSProperties = {
    transform: `translate(${appearance.titleOffsetX}px, ${appearance.titleOffsetY}px)`,
  };
  const artStyle: CSSProperties = { opacity: appearance.iconOpacity };
  return <button draggable data-testid="deck-key" className={`control key-control${selected ? ' selected' : ''}`} style={style} onClick={onSelect} onDragStart={onDragStart} onDragOver={onDragOver} onDrop={onDrop}>
    <span className="control-art" style={artStyle} aria-hidden="true">{iconUrl ? <img src={iconUrl} alt="" style={{ objectFit: objectFit(appearance.fitMode) }} /> : appearance.iconAssetId ? '◆' : ''}</span>
    {appearance.titleVisible && <span className="control-title" style={titleStyle}>{appearance.title || 'Key'}</span>}
    <small>{slot.position + 1}</small>
  </button>;
}
