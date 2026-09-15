import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import App from './App';
import { bridge } from './bridge';
import { createDefaultWorkspace } from './model/workspace';

vi.mock('./bridge', () => ({ bridge: {
  editorLoadWorkspace: vi.fn(), editorSaveWorkspace: vi.fn(), editorImportAsset: vi.fn(), editorListAssets: vi.fn(), editorAssetDataUrls: vi.fn(),
  editorExportProfile: vi.fn(), editorImportProfile: vi.fn(),
  obsSaveConfig: vi.fn(), obsStatus: vi.fn(), obsScenes: vi.fn(), obsSetScene: vi.fn(), obsToggleStream: vi.fn(), obsToggleRecord: vi.fn(), obsToggleMute: vi.fn(),
  twitchStatus: vi.fn(), twitchBeginAuth: vi.fn(), twitchPollAuth: vi.fn(),
  openExternal: vi.fn(), scanMarketplace: vi.fn(),
} }));

beforeEach(() => {
  vi.mocked(bridge.editorLoadWorkspace).mockResolvedValue({ workspace: createDefaultWorkspace(), source: 'primary', warning: null });
  vi.mocked(bridge.editorSaveWorkspace).mockResolvedValue(undefined);
  vi.mocked(bridge.editorImportAsset).mockRejectedValue(new Error('not used'));
  vi.mocked(bridge.editorListAssets).mockResolvedValue([]);
  vi.mocked(bridge.editorAssetDataUrls).mockResolvedValue({});
  vi.mocked(bridge.editorExportProfile).mockResolvedValue(undefined);
  vi.mocked(bridge.editorImportProfile).mockRejectedValue(new Error('not used'));
  vi.mocked(bridge.obsSaveConfig).mockResolvedValue(undefined);
  vi.mocked(bridge.obsStatus).mockResolvedValue('Connected');
  vi.mocked(bridge.obsScenes).mockResolvedValue([]);
  vi.mocked(bridge.obsSetScene).mockResolvedValue(undefined);
  vi.mocked(bridge.obsToggleStream).mockResolvedValue(undefined);
  vi.mocked(bridge.obsToggleRecord).mockResolvedValue(undefined);
  vi.mocked(bridge.obsToggleMute).mockResolvedValue(undefined);
  vi.mocked(bridge.twitchStatus).mockResolvedValue(null);
  vi.mocked(bridge.twitchBeginAuth).mockResolvedValue({device_code:'d',user_code:'u',verification_uri:'https://example.com',expires_in:600,interval:5});
  vi.mocked(bridge.twitchPollAuth).mockResolvedValue(null);
  vi.mocked(bridge.openExternal).mockResolvedValue(undefined);
  vi.mocked(bridge.scanMarketplace).mockResolvedValue([]);
});

afterEach(() => vi.clearAllMocks());

async function renderEditor() {
  render(<App />);
  await screen.findByTestId('deck-plus');
}

describe('OpenDeck 2.0.1 editor', () => {
  it('renders all Stream Deck Plus editable surfaces', async () => {
    await renderEditor();
    expect(screen.getAllByTestId('deck-key')).toHaveLength(8);
    expect(screen.getAllByTestId('dial')).toHaveLength(4);
    expect(screen.getAllByTestId('touch-control')).toHaveLength(4);
    expect(screen.getByText('Property Inspector')).toBeInTheDocument();
  });

  it('selects touch regions and exposes appearance customization', async () => {
    await renderEditor();
    fireEvent.click(screen.getAllByTestId('touch-control')[0]);
    expect(screen.getByText('touch 1')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Appearance' }));
    expect(screen.getByText('Show title')).toBeInTheDocument();
  });

  it('assigns an action to a selected key', async () => {
    await renderEditor();
    fireEvent.click(screen.getAllByTestId('deck-key')[0]);
    fireEvent.click(screen.getByRole('button', { name: 'Toggle Stream' }));
    expect(screen.getAllByTestId('deck-key')[0]).toHaveTextContent('Toggle Stream');
  });

  it('previews active-state appearance without changing the base appearance', async () => {
    await renderEditor();
    const key = screen.getAllByTestId('deck-key')[0];
    fireEvent.click(key);
    fireEvent.click(screen.getByRole('button', { name: 'States' }));
    fireEvent.change(screen.getByLabelText('Title'), { target: { value: 'LIVE' } });
    expect(key).toHaveTextContent('Key');
    fireEvent.click(screen.getByRole('button', { name: 'Preview active state' }));
    expect(key).toHaveTextContent('LIVE');
    fireEvent.click(screen.getByRole('button', { name: 'Preview default state' }));
    expect(key).toHaveTextContent('Key');
  });

  it('collapses the action panel and property inspector independently', async () => {
    await renderEditor();
    fireEvent.click(screen.getByRole('button', { name: 'Collapse action panel' }));
    expect(screen.getByRole('button', { name: 'Expand action panel' })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Collapse property inspector' }));
    expect(screen.getByRole('button', { name: 'Expand property inspector' })).toBeInTheDocument();
  });

  it('makes action-library entries draggable for direct assignment', async () => {
    await renderEditor();
    expect(screen.getByRole('button', { name: 'Toggle Record' })).toHaveAttribute('draggable', 'true');
  });
});
