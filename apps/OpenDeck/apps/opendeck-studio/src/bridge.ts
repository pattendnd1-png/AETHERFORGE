import { invoke } from '@tauri-apps/api/core';

export type QualificationPhase = 'visual' | 'performance' | 'startup';
export interface QualificationContext { enabled: boolean; phase: QualificationPhase; startedAtMs?: number; tauriSetupMs?: number }
export interface QualificationFocusAck { release: string; pid: number; title: string }
import { getCurrentWindow } from '@tauri-apps/api/window';
export type ResizeDirection = 'East'|'North'|'NorthEast'|'NorthWest'|'South'|'SouthEast'|'SouthWest'|'West';
import type { ActiveApplicationContext, MarketplaceItem, StreamDeckStatus, TwitchDeviceCode, TwitchIdentity } from './types';
import type { AssetRecord, Profile, Workspace, WorkspaceLoadResult } from './model/workspace';
import type { PluginDescriptor, PluginDispatchRequest, PluginHostStatus, PluginPropertyInspectorSession } from './plugins/types';

export const bridge = {
  appShow: () => getCurrentWindow().show(),
  appMinimize: () => getCurrentWindow().minimize(),
  appToggleMaximize: () => getCurrentWindow().toggleMaximize(),
  appMaximize: () => getCurrentWindow().maximize(),
  appClose: () => getCurrentWindow().close(),
  appStartDragging: () => getCurrentWindow().startDragging(),
  appStartResizeDragging: (direction: ResizeDirection) => getCurrentWindow().startResizeDragging(direction),
  startupVisibleAck: () => invoke<boolean>('startup_visible_ack'),
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
  pluginList: () => invoke<PluginDescriptor[]>('plugin_list'),
  pluginHostStatus: () => invoke<PluginHostStatus>('plugin_host_status'),
  pluginInstall: (path: string) => invoke<PluginDescriptor>('plugin_install', { path }),
  pluginRemove: (pluginUuid: string) => invoke<void>('plugin_remove', { pluginUuid }),
  pluginSetEnabled: (pluginUuid: string, enabled: boolean) => invoke<void>('plugin_set_enabled', { pluginUuid, enabled }),
  pluginSetActive: (pluginUuid: string, active: boolean) => invoke<void>('plugin_set_active', { pluginUuid, active }),
  pluginRestart: (pluginUuid: string) => invoke<void>('plugin_restart', { pluginUuid }),
  pluginDispatchAction: (request: PluginDispatchRequest) => invoke<void>('plugin_dispatch_action', { request }),
  pluginDispatchLifecycle: (request: { pluginUuid: string; actionUuid: string; context: string; device: string; controlKind: string; position: number; event: 'willAppear'|'willDisappear'; isInMultiAction?: boolean }) => invoke<void>('plugin_dispatch_lifecycle', { request }),
  pluginDispatchHostEvent: (event: string, payload: unknown = {}, device?: string, application?: string) => invoke<void>('plugin_dispatch_host_event', { request: { event, payload, device, application } }),
  pluginImportBundledProfile: (pluginUuid: string, profileName: string) => invoke<Profile>('plugin_import_bundled_profile', { pluginUuid, profileName }),
  pluginPropertyInspectorSession: (pluginUuid: string, actionUuid: string, context: string) => invoke<PluginPropertyInspectorSession>('plugin_property_inspector_session', { request: { pluginUuid, actionUuid, context } }),
  streamdeckStatus: () => invoke<StreamDeckStatus>('streamdeck_status'),
  activeApplicationContext: () => invoke<ActiveApplicationContext>('active_application_context'),
  streamdeckSyncWorkspace: (workspace: Workspace, activeAppId: string | null) => invoke<void>('streamdeck_sync_workspace', { workspace, activeAppId }),
  streamdeckSetBrightness: (percent: number) => invoke<void>('streamdeck_set_brightness', { percent }),
  qualificationContext: () => invoke<QualificationContext>('qualification_context'),
  qualificationFocusWindow: () => invoke<QualificationFocusAck>('qualification_focus_window'),
  qualificationRecordUiMetrics: (payload: unknown) => invoke<void>('qualification_record_ui_metrics', { payload }),
};
