import { memo, type CSSProperties, type DragEventHandler } from 'react';
import type { Appearance, ControlSlot } from '../model/workspace';
import { Glyph, glyphForLabel } from './Glyph';

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
function objectFit(value: ControlSlot['appearance']['fitMode']): CSSProperties['objectFit'] { return value === 'stretch' ? 'fill' : value; }
function backgroundSize(value: ControlSlot['appearance']['fitMode']): CSSProperties['backgroundSize'] { return value === 'stretch' ? '100% 100%' : value; }

export const KeyControl = memo(function KeyControl({ slot, appearance, selected, iconUrl, backgroundUrl, onSelect, onDragStart, onDragOver, onDrop }: KeyControlProps) {
  const title = appearance.title || 'Key';
  const style = {
    '--key-color': appearance.backgroundColor,
    backgroundImage: backgroundUrl ? `url(${JSON.stringify(backgroundUrl)})` : undefined,
    backgroundPosition: 'center', backgroundRepeat: 'no-repeat', backgroundSize: backgroundSize(appearance.fitMode),
    color: appearance.textColor, fontFamily: appearance.fontFamily, fontSize: `${appearance.fontSize}px`, fontWeight: appearance.fontWeight,
    textAlign: appearance.horizontalAlign, justifyContent: verticalJustify(appearance.verticalAlign),
  } as CSSProperties;
  const titleStyle: CSSProperties = { transform: `translate(${appearance.titleOffsetX}px, ${appearance.titleOffsetY}px)` };
  return <button aria-label={title} draggable data-testid="deck-key" data-qualify-element="key" className={`control key-control${selected ? ' selected' : ''}`} style={style} onClick={onSelect} onDragStart={onDragStart} onDragOver={onDragOver} onDrop={onDrop}>
    <span className="key-glow" aria-hidden="true" />
    <span className="control-art" style={{ opacity: appearance.iconOpacity }} aria-hidden="true">{iconUrl ? <img src={iconUrl} alt="" style={{ objectFit: objectFit(appearance.fitMode) }} /> : <Glyph name={glyphForLabel(title)} />}</span>
    {appearance.titleVisible && <span className="control-title" style={titleStyle}>{title}</span>}
    <small>{slot.position + 1}</small>
  </button>;
});
