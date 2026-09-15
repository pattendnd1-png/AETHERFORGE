import { invoke } from '@tauri-apps/api/core';
import type { MarketplaceItem, TwitchDeviceCode, TwitchIdentity } from './types';

export const bridge = {
  obsSaveConfig: (host: string, port: number, password: string) => invoke<void>('obs_save_config', { config: { host, port, password } }),
  obsStatus: () => invoke<string>('obs_status'),
  obsScenes: () => invoke<string[]>('obs_scene_names'),
  obsSetScene: (sceneName: string) => invoke<void>('obs_set_scene', { sceneName }),
  obsToggleStream: () => invoke<void>('obs_toggle_stream'),
  obsToggleRecord: () => invoke<void>('obs_toggle_record'),
  obsToggleMute: (inputName: string) => invoke<void>('obs_toggle_mute', { inputName }),
  twitchStatus: () => invoke<TwitchIdentity | null>('twitch_status'),
  twitchBeginAuth: () => invoke<TwitchDeviceCode>('twitch_begin_auth'),
  twitchPollAuth: (deviceCode: string) => invoke<TwitchIdentity | null>('twitch_poll_auth', { deviceCode }),
  openExternal: (url: string) => invoke<void>('open_external', { url }),
  scanMarketplace: () => invoke<MarketplaceItem[]>('scan_marketplace_downloads'),
};
