import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { AssetBrowser } from './AssetBrowser';
import type { AssetRecord } from '../model/workspace';

function assets(count: number): AssetRecord[] {
  return Array.from({ length: count }, (_, index) => ({
    id: `asset-${index}`,
    name: `Icon ${index}`,
    path: `/icons/${index}.png`,
    source: 'icon-pack' as const,
    mime: 'image/png',
    sha256: `${index}`.padStart(64, '0'),
  }));
}

describe('AssetBrowser', () => {
  it('batches visible preview requests instead of loading thumbnails one at a time', async () => {
    const loadPreviews = vi.fn().mockResolvedValue({});
    render(<AssetBrowser assets={assets(30)} role="icon" loadPreviews={loadPreviews} onPick={vi.fn()} onImport={vi.fn().mockResolvedValue(undefined)} onClose={vi.fn()} />);
    await waitFor(() => expect(loadPreviews).toHaveBeenCalledTimes(1));
    expect(loadPreviews.mock.calls[0][0]).toHaveLength(30);
  });

  it('limits the initial asset grid and expands it on demand', () => {
    render(<AssetBrowser assets={assets(130)} role="icon" loadPreviews={vi.fn().mockResolvedValue({})} onPick={vi.fn()} onImport={vi.fn().mockResolvedValue(undefined)} onClose={vi.fn()} />);
    expect(screen.getAllByRole('button', { name: /Icon \d+/ })).toHaveLength(120);
    fireEvent.click(screen.getByRole('button', { name: 'Show more' }));
    expect(screen.getAllByRole('button', { name: /Icon \d+/ })).toHaveLength(130);
  });
});
