import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { ActionWheel } from '../model/workspace';
import { ActionWheelEditor } from './ActionWheelEditor';

const wheel: ActionWheel = {
  behavior: 'rotateSelectPressExecute',
  activeIndex: 0,
  entries: [
    { id: 'one', label: 'OBS', bindings: {} },
    { id: 'two', label: 'Browser', bindings: {} },
  ],
};

describe('ActionWheelEditor', () => {
  it('shows ordered entries and selects an entry', () => {
    const onSelect = vi.fn();
    render(<ActionWheelEditor wheel={wheel} onSelect={onSelect} onAdd={vi.fn()} onRename={vi.fn()} onMove={vi.fn()} onRemove={vi.fn()} onRemoveWheel={vi.fn()} />);
    expect(screen.getByText('Action Wheel')).toBeInTheDocument();
    expect(screen.getByLabelText('Wheel entry 1 label')).toHaveValue('OBS');
    fireEvent.click(screen.getByRole('button', { name: /Select Browser/i }));
    expect(onSelect).toHaveBeenCalledWith('two');
  });

  it('exposes add/remove controls without requiring drag', () => {
    const onAdd = vi.fn();
    const onRemove = vi.fn();
    render(<ActionWheelEditor wheel={wheel} onSelect={vi.fn()} onAdd={onAdd} onRename={vi.fn()} onMove={vi.fn()} onRemove={onRemove} onRemoveWheel={vi.fn()} />);
    fireEvent.click(screen.getByRole('button', { name: /Add Action to Wheel/i }));
    expect(onAdd).toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: /Remove Browser/i }));
    expect(onRemove).toHaveBeenCalledWith('two');
  });
});
