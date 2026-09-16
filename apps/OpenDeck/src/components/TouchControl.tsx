import { memo, type CSSProperties, type DragEventHandler } from 'react';
import type { Appearance, ControlSlot } from '../model/workspace';
import { Glyph, glyphForLabel } from './Glyph';

export interface TouchControlProps {
  slot: ControlSlot; appearance: Appearance; selected: boolean; iconUrl?: string; backgroundUrl?: string; onSelect: () => void;
  onDragStart?: DragEventHandler<HTMLButtonElement>; onDragOver?: DragEventHandler<HTMLButtonElement>; onDrop?: DragEventHandler<HTMLButtonElement>;
}
function backgroundSize(value: ControlSlot['appearance']['fitMode']): CSSProperties['backgroundSize'] { return value === 'stretch' ? '100% 100%' : value; }

export const TouchControl = memo(function TouchControl({ slot, appearance, selected, iconUrl, backgroundUrl, onSelect, onDragStart, onDragOver, onDrop }: TouchControlProps) {
  const title = appearance.title || `Touch ${slot.position + 1}`;
  const style = {
    '--touch-accent': appearance.backgroundColor, backgroundImage: backgroundUrl ? `url(${JSON.stringify(backgroundUrl)})` : undefined,
    backgroundPosition: 'center', backgroundRepeat: 'no-repeat', backgroundSize: backgroundSize(appearance.fitMode),
    color: appearance.textColor, fontFamily: appearance.fontFamily, fontSize: `${Math.max(8, Math.min(18, appearance.fontSize - 2))}px`, fontWeight: appearance.fontWeight,
    textAlign: appearance.horizontalAlign,
  } as CSSProperties;
  return <button aria-label={title} draggable data-testid="touch-control" data-qualify-element="touch" className={`touch-control${selected ? ' selected' : ''}`} onClick={onSelect} onDragStart={onDragStart} onDragOver={onDragOver} onDrop={onDrop} style={style}>
    <span className="touch-icon">{iconUrl ? <img className="touch-art-image" src={iconUrl} alt="" style={{ opacity: appearance.iconOpacity }} /> : <Glyph name={glyphForLabel(title)} />}</span>
    {appearance.titleVisible && <span className="touch-title" style={{ transform: `translate(${appearance.titleOffsetX}px, ${appearance.titleOffsetY}px)` }}>{title}</span>}
  </button>;
});
