import { invoke } from '@tauri-apps/api/core';
import type { MarketplaceItem, TwitchDeviceCode, TwitchIdentity } from './types';
import type { AssetRecord, Profile, Workspace, WorkspaceLoadResult } from './model/workspace';

export const bridge = {
  editorLoadWorkspace: () => invoke<WorkspaceLoadResult>('editor_load_workspace'),
  editorSaveWorkspace: (workspace: Workspace) => invoke<void>('editor_save_workspace', { workspace }),
  editorImportAsset: (path: string) => invoke<AssetRecord>('editor_import_asset', { path }),
  editorListAssets: () => invoke<AssetRecord[]>('editor_list_assets'),
  editorAssetDataUrls: (assetIds: string[]) => invoke<Record<string, string>>('editor_asset_data_urls', { assetIds }),
  editorExportProfile: (profile: Profile, path: string) => invoke<void>('editor_export_profile', { profile, path }),
  editorImportProfile: (path: string) => invoke<Profile>('editor_import_profile', { path }),
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
