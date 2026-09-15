import { afterEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import App from './App';
import { bridge } from './bridge';

vi.mock('./bridge', () => ({ bridge: {
  obsSaveConfig: vi.fn().mockResolvedValue(undefined), obsStatus: vi.fn().mockResolvedValue('Connected'), obsScenes: vi.fn().mockResolvedValue([]),
  obsSetScene: vi.fn().mockResolvedValue(undefined), obsToggleStream: vi.fn().mockResolvedValue(undefined), obsToggleRecord: vi.fn().mockResolvedValue(undefined), obsToggleMute: vi.fn().mockResolvedValue(undefined),
  twitchStatus: vi.fn().mockResolvedValue(null), twitchBeginAuth: vi.fn().mockResolvedValue({device_code:'d',user_code:'u',verification_uri:'https://example.com',expires_in:600,interval:5}), twitchPollAuth: vi.fn().mockResolvedValue(null),
  openExternal: vi.fn().mockResolvedValue(undefined), scanMarketplace: vi.fn().mockResolvedValue([]),
} }));

afterEach(() => vi.clearAllMocks());

describe('OpenDeck 2.0.1 editor', () => {
  it('renders all Stream Deck Plus editable surfaces', () => {
    render(<App />);
    expect(screen.getAllByTestId('deck-key')).toHaveLength(8);
    expect(screen.getAllByTestId('dial')).toHaveLength(4);
    expect(screen.getAllByTestId('touch-control')).toHaveLength(4);
    expect(screen.getByText('Property Inspector')).toBeInTheDocument();
  });

  it('selects touch regions and exposes appearance customization', () => {
    render(<App />);
    fireEvent.click(screen.getAllByTestId('touch-control')[0]);
    expect(screen.getByText('touch 1')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Appearance' }));
    expect(screen.getByText('Show title')).toBeInTheDocument();
  });

  it('assigns an action to a selected key', () => {
    render(<App />);
    fireEvent.click(screen.getAllByTestId('deck-key')[0]);
    fireEvent.click(screen.getByRole('button', { name: 'Toggle Stream' }));
    expect(screen.getAllByTestId('deck-key')[0]).toHaveTextContent('Toggle Stream');
  });
});
