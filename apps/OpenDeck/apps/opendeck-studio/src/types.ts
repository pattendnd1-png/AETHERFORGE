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
