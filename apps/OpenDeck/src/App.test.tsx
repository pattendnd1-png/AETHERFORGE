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
  pluginList: vi.fn(), pluginHostStatus: vi.fn(), pluginInstall: vi.fn(), pluginRemove: vi.fn(),
  pluginSetEnabled: vi.fn(), pluginSetActive: vi.fn(), pluginRestart: vi.fn(), pluginDispatchAction: vi.fn(),
  pluginDispatchLifecycle: vi.fn(), pluginDispatchHostEvent: vi.fn(), pluginImportBundledProfile: vi.fn(), pluginPropertyInspectorSession: vi.fn(),
  streamdeckStatus: vi.fn(), activeApplicationContext: vi.fn(), streamdeckSyncWorkspace: vi.fn(), streamdeckSetBrightness: vi.fn(),
  qualificationContext: vi.fn(), qualificationFocusWindow: vi.fn(), qualificationRecordUiMetrics: vi.fn(),
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
  vi.mocked(bridge.twitchBeginAuth).mockResolvedValue({ device_code: 'd', user_code: 'u', verification_uri: 'https://example.com', expires_in: 600, interval: 5 });
  vi.mocked(bridge.twitchPollAuth).mockResolvedValue(null);
  vi.mocked(bridge.openExternal).mockResolvedValue(undefined);
  vi.mocked(bridge.scanMarketplace).mockResolvedValue([]);
  vi.mocked(bridge.pluginList).mockResolvedValue([]);
  vi.mocked(bridge.pluginHostStatus).mockResolvedValue({ protocolVersion: '2.0.40', streamDeckCompatibilityTarget: '7.6', websocketHost: '127.0.0.1', installed: 0, active: 0, enabled: 0 });
  vi.mocked(bridge.pluginInstall).mockRejectedValue(new Error('not used'));
  vi.mocked(bridge.pluginRemove).mockResolvedValue(undefined);
  vi.mocked(bridge.pluginSetEnabled).mockResolvedValue(undefined);
  vi.mocked(bridge.pluginSetActive).mockResolvedValue(undefined);
  vi.mocked(bridge.pluginRestart).mockResolvedValue(undefined);
  vi.mocked(bridge.pluginDispatchAction).mockResolvedValue(undefined);
  vi.mocked(bridge.pluginDispatchLifecycle).mockResolvedValue(undefined);
  vi.mocked(bridge.pluginDispatchHostEvent).mockResolvedValue(undefined);
  vi.mocked(bridge.pluginImportBundledProfile).mockRejectedValue(new Error('not used'));
  vi.mocked(bridge.pluginPropertyInspectorSession).mockRejectedValue(new Error('not used'));
  vi.mocked(bridge.streamdeckStatus).mockResolvedValue({ state: 'disconnected', model: 'Stream Deck +', serial: null, message: 'Not connected' });
  vi.mocked(bridge.activeApplicationContext).mockResolvedValue({ appId: null, provider: 'unavailable' });
  vi.mocked(bridge.streamdeckSyncWorkspace).mockResolvedValue(undefined);
  vi.mocked(bridge.streamdeckSetBrightness).mockResolvedValue(undefined);
  vi.mocked(bridge.qualificationContext).mockResolvedValue({ enabled: false, phase: 'visual' });
  vi.mocked(bridge.qualificationFocusWindow).mockResolvedValue({ release: '2.0.40', pid: 1234, title: 'OpenDeck+ 2.0.40 Qualification [1234]' });
  vi.mocked(bridge.qualificationRecordUiMetrics).mockResolvedValue(undefined);
});

afterEach(() => vi.clearAllMocks());

async function renderEditor() {
  render(<App />);
  await screen.findByTestId('deck-plus');
}

describe('OpenDeck 2.0.40 editor', () => {
  it('accepts qualification context directly without depending on a URL mutation', async () => {
    window.history.replaceState({}, '', '/');
    render(<App qualification={{ enabled: true, phase: 'visual' }} />);
    await screen.findByTestId('deck-plus');
    expect(bridge.editorLoadWorkspace).not.toHaveBeenCalled();
    expect(screen.getByTitle('Connected')).toHaveClass('hardware-status-dot');
  });

  it('renders the Windows editor hierarchy and all Stream Deck Plus surfaces', async () => {
    await renderEditor();
    expect(screen.getAllByTestId('deck-key')).toHaveLength(8);
    expect(screen.getAllByTestId('dial')).toHaveLength(4);
    expect(screen.getAllByTestId('touch-control')).toHaveLength(4);
    expect(screen.getByRole('tab', { name: 'Keys' })).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Dials' })).toBeInTheDocument();
    expect(screen.queryByText('Property Inspector')).not.toBeInTheDocument();
    expect(document.querySelector('.statusbar')).toBeNull();
    expect(document.querySelector('[data-layout-region="configuration"]')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Page 1' })).not.toBeInTheDocument();
  });

  it('switches an adaptive touch strip to unified for a matching active application', async () => {
    const workspace = createDefaultWorkspace();
    const touchStrip = workspace.profiles[0].pages[0].touchStrip;
    touchStrip.mode = 'adaptive';
    touchStrip.adaptiveFallback = 'segmented';
    touchStrip.adaptiveRules = [{ pattern: 'spotify', presentation: 'unified' }];
    vi.mocked(bridge.editorLoadWorkspace).mockResolvedValue({ workspace, source: 'primary', warning: null });
    vi.mocked(bridge.activeApplicationContext).mockResolvedValue({ appId: 'com.spotify.Client', provider: 'environment' });

    await renderEditor();
    await screen.findByTestId('unified-touch-control');
    expect(bridge.activeApplicationContext).toHaveBeenCalled();
    await waitFor(() => expect(bridge.streamdeckSyncWorkspace).toHaveBeenCalledWith(expect.anything(), 'com.spotify.Client'));
  });

  it('uses compact device/profile controls without the old connection pill', async () => {
    vi.mocked(bridge.streamdeckStatus).mockResolvedValue({ state: 'connected', model: 'Stream Deck +', serial: 'ABC123', message: 'Connected' });
    await renderEditor();
    expect(screen.getByLabelText('Device')).toHaveValue('Stream Deck +');
    expect(screen.getByLabelText('Profile')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Profile options' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Connections' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Marketplace' })).toBeInTheDocument();
    expect(screen.queryByText('Stream Deck + · Connected')).not.toBeInTheDocument();
    expect(screen.getByTitle('Connected')).toHaveClass('hardware-status-dot');
  });

  it('automatically switches Keys and Dials modes with control selection', async () => {
    await renderEditor();
    fireEvent.click(screen.getAllByTestId('dial')[0]);
    expect(screen.getByRole('tab', { name: 'Dials' })).toHaveAttribute('aria-selected', 'true');
    fireEvent.click(screen.getAllByTestId('deck-key')[0]);
    expect(screen.getByRole('tab', { name: 'Keys' })).toHaveAttribute('aria-selected', 'true');
  });

  it('filters non-dial actions out of Dials mode', async () => {
    await renderEditor();
    expect(screen.getByRole('button', { name: 'Folder' })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('tab', { name: 'Dials' }));
    expect(screen.queryByRole('button', { name: 'Folder' })).not.toBeInTheDocument();
  });

  it('creates and edits a Dial Stack from the Dials library', async () => {
    await renderEditor();
    fireEvent.click(screen.getAllByTestId('dial')[0]);
    fireEvent.click(screen.getByRole('button', { name: 'Dial Stack' }));
    expect(screen.getByLabelText('Dial Stack editor')).toBeInTheDocument();
    expect(screen.getByText('Press cycles the active entry')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '+ Add Action to Stack' }));
    expect(screen.getByLabelText('Stack entry 2 label')).toHaveValue('Entry 2');
    fireEvent.change(screen.getByLabelText('Stack entry 2 label'), { target: { value: 'Streaming' } });
    expect(screen.getByLabelText('Stack entry 2 label')).toHaveValue('Streaming');
  });

  it('creates an Action Wheel and edits its selected action', async () => {
    await renderEditor();
    fireEvent.click(screen.getAllByTestId('dial')[1]);
    fireEvent.click(screen.getByRole('button', { name: 'Action Wheel' }));
    expect(screen.getByLabelText('Action Wheel editor')).toBeInTheDocument();
    expect(screen.getByText('Selects wheel action')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '+ Add Action to Wheel' }));
    expect(screen.getByLabelText('Wheel entry 2 label')).toHaveValue('Action 2');
    fireEvent.click(screen.getByRole('button', { name: 'Toggle Record' }));
    expect(screen.getByText('Selected Wheel Action')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Test Action' })).toBeInTheDocument();
  });

  it('executes the selected Action Wheel entry on press/test', async () => {
    await renderEditor();
    fireEvent.click(screen.getAllByTestId('dial')[1]);
    fireEvent.click(screen.getByRole('button', { name: 'Action Wheel' }));
    fireEvent.click(screen.getByRole('button', { name: 'Open Marketplace' }));
    fireEvent.click(screen.getByRole('button', { name: 'Test Action' }));
    await waitFor(() => expect(bridge.openExternal).toHaveBeenCalledWith('https://marketplace.elgato.com'));
  });

  it('assigns actions to the selected dial interaction including press-plus-rotate', async () => {
    await renderEditor();
    fireEvent.click(screen.getAllByTestId('dial')[0]);
    fireEvent.change(screen.getByLabelText('Interaction'), { target: { value: 'pressRotateLeft' } });
    expect(screen.queryByRole('button', { name: 'Test Action' })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Previous Page' }));
    expect(screen.getByRole('button', { name: 'Test Action' })).toBeInTheDocument();
  });

  it('uses numbered page navigation and adds pages', async () => {
    const workspace = createDefaultWorkspace();
    const profile = workspace.profiles[0];
    const second = structuredClone(profile.pages[0]);
    second.id = 'page-2';
    second.name = 'Page 2';
    profile.pages.push(second);
    vi.mocked(bridge.editorLoadWorkspace).mockResolvedValue({ workspace, source: 'primary', warning: null });

    await renderEditor();
    expect(screen.getByRole('button', { name: 'Page 1' })).toHaveAttribute('aria-pressed', 'true');
    fireEvent.click(screen.getByRole('button', { name: 'Page 2' }));
    expect(screen.getByRole('button', { name: 'Page 2' })).toHaveAttribute('aria-pressed', 'true');
    fireEvent.click(screen.getByRole('button', { name: 'Add page' }));
    expect(screen.getByRole('button', { name: 'Page 3' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Page options' })).toBeInTheDocument();
  });

  it('selects touch regions and exposes appearance customization', async () => {
    await renderEditor();
    fireEvent.click(screen.getAllByTestId('touch-control')[0]);
    expect(screen.getByText('touch 1')).toBeInTheDocument();
    expect(screen.getByRole('tab', { name: 'Dials' })).toHaveAttribute('aria-selected', 'true');
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

  it('keeps the configuration strip below the device and its empty state quiet', async () => {
    await renderEditor();
    const device = screen.getByTestId('deck-plus').closest('[data-layout-region="device"]');
    const configuration = document.querySelector('[data-layout-region="configuration"]');
    const actions = screen.getByRole('complementary', { name: 'Actions' });
    expect(screen.getByText('Select a key, dial, or touch region to configure it.')).toBeInTheDocument();
    expect(device).toBeInTheDocument();
    expect(configuration).toBeInTheDocument();
    expect(actions).toHaveAttribute('data-layout-region', 'actions');
    expect(device?.closest('.editor-column')).toBe(configuration?.closest('.editor-column'));
    expect(actions.parentElement).toHaveClass('workspace-grid');
  });

  it('collapses the action panel and configuration independently', async () => {
    await renderEditor();
    fireEvent.click(screen.getByRole('button', { name: 'Collapse action panel' }));
    expect(screen.getByRole('button', { name: 'Expand action panel' })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Collapse configuration' }));
    expect(screen.getByRole('button', { name: 'Expand configuration' })).toBeInTheDocument();
  });

  it('removes the permanent footer and exposes save/status feedback in-shell', async () => {
    await renderEditor();
    expect(document.querySelector('.statusbar')).toBeNull();
    expect(screen.getByLabelText('Save status: Saved')).toBeInTheDocument();
    fireEvent.click(screen.getAllByTestId('deck-key')[0]);
    expect(screen.getByRole('status')).toBeInTheDocument();
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
});
