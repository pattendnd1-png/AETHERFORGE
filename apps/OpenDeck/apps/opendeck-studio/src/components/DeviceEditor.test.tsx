import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { createQualificationWorkspace } from '../qualify/demoWorkspace';
import { getActivePage } from '../model/workspace';
import { DeviceEditor } from './DeviceEditor';

describe('DeviceEditor v2.0.29 physical presentation', () => {
  it('renders one coherent Stream Deck Plus body', () => {
    const page = getActivePage(createQualificationWorkspace());
    render(<DeviceEditor page={page} selection={null} assetPreviews={{}} previewState="default" touchPresentation="segmented" onSelect={vi.fn()} onDropControl={vi.fn()} onDropAction={vi.fn()} />);
    expect(screen.getByTestId('deck-plus')).toHaveClass('streamdeck-device');
    expect(screen.getAllByTestId('deck-key')).toHaveLength(8);
    expect(screen.getByRole('button', { name: 'Key 1: Scene' })).toBeInTheDocument();
    expect(screen.getByTestId('touch-display')).toContainElement(screen.getAllByTestId('touch-control')[0]);
    expect(screen.getAllByTestId('dial')).toHaveLength(4);
    expect(document.querySelectorAll('.dial-ring')).toHaveLength(4);
  });

  it('renders the same bezel as one full-width control in unified mode', () => {
    const page = getActivePage(createQualificationWorkspace());
    render(<DeviceEditor page={page} selection={null} assetPreviews={{}} previewState="default" touchPresentation="unified" onSelect={vi.fn()} onDropControl={vi.fn()} onDropAction={vi.fn()} />);
    expect(screen.getByTestId('touch-display')).toHaveClass('unified');
    expect(screen.getByTestId('unified-touch-control')).toBeInTheDocument();
    expect(screen.queryAllByTestId('touch-control')).toHaveLength(0);
  });
});
