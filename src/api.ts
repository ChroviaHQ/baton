import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export type ProxyProtocol = 'http' | 'socks5';

export type UpstreamMode = 'manual' | 'system';

export type HopRole = 'upstream' | 'landing';

export type Hop = {
  protocol: ProxyProtocol;
  host: string;
  port: number;
  username: string;
  password: string;
};

export type AppConfig = {
  upstreamMode: UpstreamMode;
  upstream: Hop;
  landing: Hop;
  listenPort: number;
  allowLan: boolean;
  useAsSystemProxy: boolean;
  startOnLaunch: boolean;
  showInDock: boolean;
  bypass: string[];
};

export type LanAddress = {interface: string; ip: string;};

export type Status = {config: AppConfig; running: boolean;};

export type TitleBarMetrics = {height: number; controlsWidth: number;};

export type Stats = {active: number; failed: number; upRate: number; downRate: number;};

export type EgressInfo = {
  ip: string;
  country: string;
  region: string;
  city: string;
  org: string;
  latencyMs: number;
};

export type CommandError = {
  code: string;
  hop: number | null;
  detail: string;
  reply: number | null;
};

export function toCommandError(error: unknown): CommandError {
  if (error && typeof error === 'object' && 'code' in error) {
    return error as CommandError;
  }
  return {code: 'unknown', hop: null, detail: String(error), reply: null};
}

export const api = {
  getStatus: () => invoke<Status>('get_status'),
  getLanAddresses: () => invoke<LanAddress[]>('get_lan_addresses'),
  startRelay: () => invoke<Status>('start_relay'),
  stopRelay: () => invoke<Status>('stop_relay'),
  saveConfig: (config: AppConfig) => invoke<Status>('save_config', {config}),
  probeEgress: (config: AppConfig, through: HopRole) =>
    invoke<EgressInfo>('probe_egress', {config, through}),
  detectSystemProxy: () => invoke<Hop | null>('detect_system_proxy'),
  getTitleBarMetrics: () => invoke<TitleBarMetrics | null>('get_title_bar_metrics'),
  takePendingError: () => invoke<CommandError | null>('take_pending_error'),
  getLaunchAtLogin: () => invoke<boolean>('get_launch_at_login'),
  setLaunchAtLogin: (enabled: boolean) => invoke<boolean>('set_launch_at_login', {enabled}),
  getPendingUpdate: () => invoke<string | null>('get_pending_update'),
  installUpdate: () => invoke<void>('install_update'),
  onStatus: (handler: (status: Status) => void): Promise<UnlistenFn> =>
    listen<Status>('relay://status', event => handler(event.payload)),
  onStats: (handler: (stats: Stats) => void): Promise<UnlistenFn> =>
    listen<Stats>('relay://stats', event => handler(event.payload)),
  onUpdate: (handler: (version: string) => void): Promise<UnlistenFn> =>
    listen<string>('app://update', event => handler(event.payload)),
  onTitleBarMetrics: (handler: (metrics: TitleBarMetrics) => void): Promise<UnlistenFn> =>
    listen<TitleBarMetrics>('window://title-bar-metrics', event => handler(event.payload)),
  onError: (handler: (error: CommandError) => void): Promise<UnlistenFn> =>
    listen<CommandError>('app://error', event => handler(event.payload)),
};
