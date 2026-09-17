import { memo, type CSSProperties, type DragEventHandler } from 'react';
import type { Appearance, ControlSlot } from '../model/workspace';
import { Glyph, glyphForLabel } from './Glyph';

export interface UnifiedTouchControlProps {
  slot: ControlSlot;
  appearance: Appearance;
  selected: boolean;
  iconUrl?: string;
  backgroundUrl?: string;
  stackStatus?: string;
  onSelect: () => void;
  onDragStart?: DragEventHandler<HTMLButtonElement>;
  onDragOver?: DragEventHandler<HTMLButtonElement>;
  onDrop?: DragEventHandler<HTMLButtonElement>;
}

function backgroundSize(value: ControlSlot['appearance']['fitMode']): CSSProperties['backgroundSize'] {
  return value === 'stretch' ? '100% 100%' : value;
}

export const UnifiedTouchControl = memo(function UnifiedTouchControl({
  appearance,
  selected,
  iconUrl,
  backgroundUrl,
  stackStatus,
  onSelect,
  onDragStart,
  onDragOver,
  onDrop,
}: UnifiedTouchControlProps) {
  const title = appearance.title || 'Touch Strip';
  const style = {
    '--touch-accent': appearance.backgroundColor,
    backgroundImage: backgroundUrl ? `url(${JSON.stringify(backgroundUrl)})` : undefined,
    backgroundPosition: 'center',
    backgroundRepeat: 'no-repeat',
    backgroundSize: backgroundSize(appearance.fitMode),
    color: appearance.textColor,
    fontFamily: appearance.fontFamily,
    fontSize: `${Math.max(8, Math.min(18, appearance.fontSize - 2))}px`,
    fontWeight: appearance.fontWeight,
    textAlign: appearance.horizontalAlign,
  } as CSSProperties;

  return <button
    aria-label={stackStatus ? `${title}, ${stackStatus}` : title}
    draggable
    data-testid="unified-touch-control"
    data-qualify-element="touch"
    className={`touch-control unified-touch-control${selected ? ' selected' : ''}`}
    onClick={onSelect}
    onDragStart={onDragStart}
    onDragOver={onDragOver}
    onDrop={onDrop}
    style={style}
  >
    <span className="touch-icon">{iconUrl ? <img className="touch-art-image" src={iconUrl} alt="" style={{ opacity: appearance.iconOpacity }} /> : <Glyph name={glyphForLabel(title)} />}</span>
    {stackStatus ? <span className="touch-stack-status unified"><strong>{stackStatus}</strong></span> : appearance.titleVisible && <span className="touch-title" style={{ transform: `translate(${appearance.titleOffsetX}px, ${appearance.titleOffsetY}px)` }}>{title}</span>}
  </button>;
});
