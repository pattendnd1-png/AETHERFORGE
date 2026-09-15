import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
vi.mock('./bridge', () => ({ bridge: {
  obsSaveConfig: vi.fn(), obsStatus: vi.fn().mockResolvedValue('OBS 31 / WebSocket 5'), obsScenes: vi.fn().mockResolvedValue(['Scene']),
  obsSetScene: vi.fn(), obsToggleStream: vi.fn(), obsToggleRecord: vi.fn(), obsToggleMute: vi.fn(),
  twitchStatus: vi.fn().mockResolvedValue(null), twitchBeginAuth: vi.fn(), twitchPollAuth: vi.fn(), openExternal: vi.fn(), scanMarketplace: vi.fn().mockResolvedValue([]),
}}));
import App from './App';

afterEach(() => {
  cleanup();
  // Preserve module-level bridge mock implementations across tests.
  // restoreAllMocks() resets their mockResolvedValue contracts to undefined.
  vi.clearAllMocks();
});

describe('clean Windows-style baseline', () => {
  it('renders device editor, full action list, and bottom property inspector', async () => {
    render(<App />);
    expect(screen.getByRole('combobox', { name: /device/i })).toHaveValue('Stream Deck +');
    expect(screen.getByRole('heading', { name: 'Actions' })).toBeInTheDocument();
    expect(screen.getByTestId('deck-plus')).toBeInTheDocument();
    expect(screen.getByRole('heading', { name: 'Property Inspector' })).toBeInTheDocument();
    expect(screen.getAllByTestId('deck-key')).toHaveLength(8);
    expect(screen.getAllByTestId('dial')).toHaveLength(4);
  });

  it('remains usable when browser storage is unavailable', () => {
    const getItem = vi.spyOn(window.Storage.prototype, 'getItem').mockImplementation(() => { throw new Error('storage blocked'); });
    const setItem = vi.spyOn(window.Storage.prototype, 'setItem').mockImplementation(() => { throw new Error('storage blocked'); });

    try {
      render(<App />);

      expect(screen.getByTestId('deck-plus')).toBeInTheDocument();
      expect(screen.getAllByTestId('deck-key')).toHaveLength(8);
    } finally {
      getItem.mockRestore();
      setItem.mockRestore();
    }
  });

  it('assigns a selected action to a selected key', async () => {
    render(<App />);
    fireEvent.click(screen.getByRole('button', { name: /toggle stream/i }));
    fireEvent.click(screen.getAllByTestId('deck-key')[0]);
    expect(screen.getAllByTestId('deck-key')[0]).toHaveTextContent('Toggle Stream');
  });
});
