import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { DialStack } from '../model/workspace';
import { DialStackEditor } from './DialStackEditor';

const stack: DialStack = {
  behavior: 'pressCycle',
  activeIndex: 0,
  entries: [
    { id: 'one', label: 'Volume', bindings: {} },
    { id: 'two', label: 'OBS Studio', bindings: {} },
  ],
};

describe('DialStackEditor', () => {
  it('shows ordered entries and selects an entry', () => {
    const onSelect = vi.fn();
    render(<DialStackEditor stack={stack} onSelect={onSelect} onAdd={vi.fn()} onRename={vi.fn()} onMove={vi.fn()} onRemove={vi.fn()} onRemoveStack={vi.fn()} />);
    expect(screen.getByText('Dial Stack')).toBeInTheDocument();
    expect(screen.getByText('Volume')).toBeInTheDocument();
    expect(screen.getByText('OBS Studio')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /Select OBS Studio/i }));
    expect(onSelect).toHaveBeenCalledWith('two');
  });

  it('can add and remove stack entries without drag-only controls', () => {
    const onAdd = vi.fn();
    const onRemove = vi.fn();
    render(<DialStackEditor stack={stack} onSelect={vi.fn()} onAdd={onAdd} onRename={vi.fn()} onMove={vi.fn()} onRemove={onRemove} onRemoveStack={vi.fn()} />);
    fireEvent.click(screen.getByRole('button', { name: /Add action to stack/i }));
    expect(onAdd).toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: /Remove OBS Studio/i }));
    expect(onRemove).toHaveBeenCalledWith('two');
  });
});
