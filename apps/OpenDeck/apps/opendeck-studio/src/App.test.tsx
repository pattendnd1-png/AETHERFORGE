import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import App from './App';
import { bridge } from './bridge';
import { createDefaultWorkspace } from './model/workspace';

vi.mock('@tauri-apps/api/event', () => ({ listen: vi.fn().mockResolvedValue(() => undefined) }));

vi.mock('./bridge', () => ({ bridge: {
  editorLoadWorkspace: vi.fn(), editorSaveWorkspace: vi.fn(), editorImportAsset: vi.fn(), editorListAssets: vi.fn(), editorAssetDataUrls: vi.fn(),
  editorExportProfile: vi.fn(), editorImportProfile: vi.fn(),
  obsSaveConfig: vi.fn(), obsStatus: vi.fn(), obsScenes: vi.fn(), obsSetScene: vi.fn(), obsToggleStream: vi.fn(), obsToggleRecord: vi.fn(), obsToggleMute: vi.fn(),
  twitchStatus: vi.fn(), twitchBeginAuth: vi.fn(), twitchPollAuth: vi.fn(),
  openExternal: vi.fn(), scanMarketplace: vi.fn(),
  streamdeckStatus: vi.fn(), streamdeckSyncWorkspace: vi.fn(), streamdeckSetBrightness: vi.fn(),
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
  vi.mocked(bridge.streamdeckStatus).mockResolvedValue({ state: 'disconnected', model: 'Stream Deck +', serial: null, message: 'Not connected' });
  vi.mocked(bridge.streamdeckSyncWorkspace).mockResolvedValue(undefined);
  vi.mocked(bridge.streamdeckSetBrightness).mockResolvedValue(undefined);
});

afterEach(() => vi.clearAllMocks());

async function renderEditor() {
  render(<App />);
  await screen.findByTestId('deck-plus');
}

describe('OpenDeck 2.0.5 editor', () => {
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

  it('hydrates an existing Twitch identity into Connections', async () => {
    vi.mocked(bridge.twitchStatus).mockResolvedValue({ login: 'streamer', user_id: '42', expires_in: 3600 });
    await renderEditor();
    fireEvent.click(screen.getByRole('button', { name: 'Connections' }));
    expect(await screen.findByText('streamer')).toBeInTheDocument();
    expect(bridge.twitchStatus).toHaveBeenCalledTimes(1);
  });

  it('completes Twitch device sign-in by polling until an identity is returned', async () => {
    vi.mocked(bridge.twitchBeginAuth).mockResolvedValue({
      device_code: 'device-code',
      user_code: 'ABCD-EFGH',
      verification_uri: 'https://www.twitch.tv/activate',
      expires_in: 30,
      interval: 0,
    });
    vi.mocked(bridge.twitchPollAuth)
      .mockResolvedValueOnce(null)
      .mockResolvedValueOnce({ login: 'streamer', user_id: '42', expires_in: 3600 });

    await renderEditor();
    fireEvent.click(screen.getByRole('button', { name: 'Connections' }));
    fireEvent.click(screen.getByRole('button', { name: 'Sign in with Twitch' }));

    expect(await screen.findByText('Waiting for Twitch…')).toBeInTheDocument();
    expect(screen.getByText('ABCD-EFGH')).toBeInTheDocument();
    await waitFor(() => expect(bridge.twitchPollAuth).toHaveBeenCalledTimes(2));
    expect(await screen.findByText('streamer')).toBeInTheDocument();
    expect(bridge.twitchPollAuth).toHaveBeenNthCalledWith(1, 'device-code');
    expect(bridge.openExternal).toHaveBeenCalledWith('https://www.twitch.tv/activate');
  });

  it('lets the user cancel a pending Twitch device sign-in', async () => {
    vi.mocked(bridge.twitchBeginAuth).mockResolvedValue({
      device_code: 'device-code',
      user_code: 'ABCD-EFGH',
      verification_uri: 'https://www.twitch.tv/activate',
      expires_in: 30,
      interval: 60,
    });

    await renderEditor();
    fireEvent.click(screen.getByRole('button', { name: 'Connections' }));
    fireEvent.click(screen.getByRole('button', { name: 'Sign in with Twitch' }));
    expect(await screen.findByText('ABCD-EFGH')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Cancel Twitch sign-in' }));
    expect(screen.queryByText('ABCD-EFGH')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Sign in with Twitch' })).toBeEnabled();
  });

  it('expires a Twitch device sign-in when the device code lifetime elapses', async () => {
    vi.mocked(bridge.twitchBeginAuth).mockResolvedValue({
      device_code: 'device-code',
      user_code: 'ABCD-EFGH',
      verification_uri: 'https://www.twitch.tv/activate',
      expires_in: 0,
      interval: 5,
    });

    await renderEditor();
    fireEvent.click(screen.getByRole('button', { name: 'Connections' }));
    fireEvent.click(screen.getByRole('button', { name: 'Sign in with Twitch' }));
    expect(await screen.findByText('Twitch sign-in expired. Start again to get a new code.')).toBeInTheDocument();
    expect(bridge.twitchPollAuth).not.toHaveBeenCalled();
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

  it('tests a configured OBS scene only after an explicit Test Action click', async () => {
    await renderEditor();
    fireEvent.click(screen.getAllByTestId('deck-key')[0]);
    fireEvent.click(screen.getByRole('button', { name: 'Scene' }));
    fireEvent.change(screen.getByLabelText('Scene'), { target: { value: 'Gameplay' } });
    expect(bridge.obsSetScene).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: 'Test Action' }));
    await waitFor(() => expect(bridge.obsSetScene).toHaveBeenCalledWith('Gameplay'));
  });

  it('requires confirmation before a Test Action can toggle streaming', async () => {
    const confirm = vi.spyOn(window, 'confirm').mockReturnValue(false);
    await renderEditor();
    fireEvent.click(screen.getAllByTestId('deck-key')[0]);
    fireEvent.click(screen.getByRole('button', { name: 'Toggle Stream' }));
    fireEvent.click(screen.getByRole('button', { name: 'Test Action' }));
    expect(confirm).toHaveBeenCalledWith('Toggle OBS streaming now?');
    expect(bridge.obsToggleStream).not.toHaveBeenCalled();
    confirm.mockRestore();
  });

  it('keeps the Windows-style action library on the right of the editor and inspector below the device', async () => {
    await renderEditor();
    const device = screen.getByTestId('deck-plus').closest('[data-layout-region="device"]');
    const inspector = screen.getByText('Property Inspector').closest('[data-layout-region="inspector"]');
    const actions = screen.getByRole('complementary', { name: 'Actions' });
    expect(device).toBeInTheDocument();
    expect(inspector).toBeInTheDocument();
    expect(actions).toHaveAttribute('data-layout-region', 'actions');
    expect(device?.closest('.editor-column')).toBe(inspector?.closest('.editor-column'));
    expect(actions.parentElement).toHaveClass('workspace-grid');
  });

  it('shows the connected Stream Deck Plus status in the top bar', async () => {
    vi.mocked(bridge.streamdeckStatus).mockResolvedValue({ state: 'connected', model: 'Stream Deck +', serial: 'ABC123', message: 'Connected' });
    await renderEditor();
    expect(await screen.findByText('Stream Deck + · Connected')).toBeInTheDocument();
  });

});
