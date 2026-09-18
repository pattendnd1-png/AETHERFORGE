import { memo, useMemo, useState, type ReactNode, type UIEvent } from 'react';

interface VirtualActionListProps<T> {
  items: readonly T[];
  rowHeight: number;
  viewportHeight: number;
  overscan?: number;
  renderRow: (item: T, index: number) => ReactNode;
}

function VirtualActionListInner<T>({ items, rowHeight, viewportHeight, overscan = 6, renderRow }: VirtualActionListProps<T>) {
  const [scrollTop, setScrollTop] = useState(0);
  const visible = Math.ceil(viewportHeight / rowHeight);
  const start = Math.max(0, Math.floor(scrollTop / rowHeight) - overscan);
  const end = Math.min(items.length, start + visible + overscan * 2);
  const rows = useMemo(() => items.slice(start, end), [items, start, end]);
  function onScroll(event: UIEvent<HTMLDivElement>) { setScrollTop(event.currentTarget.scrollTop); }
  return <div className="virtual-action-viewport" style={{ height: viewportHeight }} onScroll={onScroll}>
    <div className="virtual-action-spacer" style={{ height: items.length * rowHeight }}>
      {rows.map((item, offset) => { const index = start + offset; return <div key={index} data-virtual-row className="virtual-action-row" style={{ transform: `translateY(${index * rowHeight}px)`, height: rowHeight }}>{renderRow(item, index)}</div>; })}
    </div>
  </div>;
}

export const VirtualActionList = memo(VirtualActionListInner) as typeof VirtualActionListInner;
