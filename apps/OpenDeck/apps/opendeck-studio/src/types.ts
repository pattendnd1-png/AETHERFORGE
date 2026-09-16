export type ServiceState = 'disconnected' | 'connecting' | 'connected' | 'error';

export type DeckAction =
  | { kind: 'obs.scene'; label: string; sceneName: string }
  | { kind: 'obs.toggleStream'; label: string }
  | { kind: 'obs.toggleRecord'; label: string }
  | { kind: 'obs.toggleMute'; label: string; inputName: string }
  | { kind: 'marketplace.open'; label: string };

export interface TwitchDeviceCode {
  device_code: string;
  user_code: string;
  verification_uri: string;
  expires_in: number;
  interval: number;
}

export interface TwitchIdentity {
  login: string;
  user_id: string;
  expires_in: number;
}

export interface MarketplaceItem { name: string; path: string; kind: string }

export interface StreamDeckStatus {
  state: 'disconnected' | 'connecting' | 'connected' | 'permissionDenied' | 'ioError';
  model: string | null;
  serial: string | null;
  message: string;
}

export type StreamDeckInputEvent =
  | { kind: 'keyDown' | 'keyUp'; index: number }
  | { kind: 'dialDown' | 'dialUp'; index: number }
  | { kind: 'dialRotate'; index: number; ticks: number; pressed: boolean }
  | { kind: 'touchTap' | 'touchPress'; x: number; y: number; region: number }
  | { kind: 'touchFlick'; startX: number; startY: number; endX: number; endY: number; region: number };
