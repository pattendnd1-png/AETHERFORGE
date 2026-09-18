import { memo, type PointerEvent } from 'react';
import { bridge, type ResizeDirection } from '../bridge';

const HANDLES: Array<{ direction: ResizeDirection; className: string; label: string }> = [
  { direction: 'North', className: 'north', label: 'Resize top edge' },
  { direction: 'South', className: 'south', label: 'Resize bottom edge' },
  { direction: 'East', className: 'east', label: 'Resize right edge' },
  { direction: 'West', className: 'west', label: 'Resize left edge' },
  { direction: 'NorthEast', className: 'north-east', label: 'Resize top-right corner' },
  { direction: 'NorthWest', className: 'north-west', label: 'Resize top-left corner' },
  { direction: 'SouthEast', className: 'south-east', label: 'Resize bottom-right corner' },
  { direction: 'SouthWest', className: 'south-west', label: 'Resize bottom-left corner' },
];

export const WindowResizeFrame = memo(function WindowResizeFrame() {
  const begin = (direction: ResizeDirection) => (event: PointerEvent<HTMLDivElement>) => {
    if (event.button !== 0) return;
    event.preventDefault();
    event.stopPropagation();
    void bridge.appStartResizeDragging(direction).catch((error) => console.error('Resize drag failed', error));
  };

  return <div className="window-resize-frame" aria-hidden="true">
    {HANDLES.map((handle) => <div
      key={handle.direction}
      className={`window-resize-handle ${handle.className}`}
      data-resize-direction={handle.direction}
      title={handle.label}
      onPointerDown={begin(handle.direction)}
    />)}
  </div>;
});
