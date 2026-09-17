import { memo, type CSSProperties, type DragEventHandler } from 'react';
import { dialStackStatus } from '../model/dial-stack';
import type { Appearance, ControlSlot } from '../model/workspace';
import { Glyph, glyphForLabel } from './Glyph';

export interface DialControlProps {
  slot: ControlSlot; appearance: Appearance; selected: boolean; iconUrl?: string; backgroundUrl?: string; onSelect: () => void;
  onDragStart?: DragEventHandler<HTMLButtonElement>; onDragOver?: DragEventHandler<HTMLButtonElement>; onDrop?: DragEventHandler<HTMLButtonElement>;
}
function backgroundSize(value: ControlSlot['appearance']['fitMode']): CSSProperties['backgroundSize'] { return value === 'stretch' ? '100% 100%' : value; }

export const DialControl = memo(function DialControl({ slot, appearance, selected, iconUrl, backgroundUrl, onSelect, onDragStart, onDragOver, onDrop }: DialControlProps) {
  const stack = dialStackStatus(slot);
  const title = stack?.label || appearance.title || `Dial ${slot.position + 1}`;
  const knobStyle = {
    '--dial-accent': appearance.backgroundColor,
    backgroundImage: backgroundUrl ? `url(${JSON.stringify(backgroundUrl)})` : undefined,
    backgroundPosition: 'center', backgroundRepeat: 'no-repeat', backgroundSize: backgroundSize(appearance.fitMode),
  } as CSSProperties;
  return <button aria-label={stack ? `${title}, Dial Stack ${stack.index} of ${stack.total}` : title} draggable data-testid="dial" data-qualify-element="dial" className={`dial-control${selected ? ' selected' : ''}${stack ? ' stacked' : ''}`} onClick={onSelect} onDragStart={onDragStart} onDragOver={onDragOver} onDrop={onDrop}>
    <span className="dial-ring" style={knobStyle}><span className="dial-cap"><span className="dial-highlight" />{iconUrl ? <img src={iconUrl} alt="" style={{ opacity: appearance.iconOpacity }} /> : <Glyph name={glyphForLabel(title)} />}{stack && <span className="dial-stack-badge">{stack.index}/{stack.total}</span>}</span></span>
    {appearance.titleVisible && <span className="dial-label" style={{ color: appearance.textColor, fontFamily: appearance.fontFamily, fontWeight: appearance.fontWeight }}>{title}</span>}
  </button>;
});
