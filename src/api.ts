import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export type ProxyKind = 'http' | 'socks5';

export type UpstreamMode = 'manual' | 'system';

export type Hop = {
  kind: ProxyKind;
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
  systemProxy: boolean;
  autoConnect: boolean;
  showInDock: boolean;
};

export type Status = {
  config: AppConfig;
  running: boolean;
  lanIp: string | null;
};

export type Stats = {
  active: number;
  failed: number;
  upRate: number;
  downRate: number;
};

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
};

export function toCommandError(error: unknown): CommandError {
  if (error && typeof error === 'object' && 'code' in error) {
    return error as CommandError;
  }
  return { code: 'unknown', hop: null, detail: String(error) };
}

export const api = {
  getStatus: () => invoke<Status>('get_status'),
  start: () => invoke<Status>('start_relay'),
  stop: () => invoke<Status>('stop_relay'),
  saveConfig: (config: AppConfig) => invoke<Status>('save_config', { config }),
  probe: (hops: Hop[]) => invoke<EgressInfo>('probe', { hops }),
  detectSystemProxy: () => invoke<Hop>('detect_system_proxy'),
  getLaunchAtLogin: () => invoke<boolean>('get_launch_at_login'),
  setLaunchAtLogin: (enabled: boolean) => invoke<boolean>('set_launch_at_login', { enabled }),
  onStatus: (handler: (status: Status) => void): Promise<UnlistenFn> =>
    listen<Status>('relay://status', event => handler(event.payload)),
  onStats: (handler: (stats: Stats) => void): Promise<UnlistenFn> =>
    listen<Stats>('relay://stats', event => handler(event.payload)),
  onError: (handler: (error: CommandError) => void): Promise<UnlistenFn> =>
    listen<CommandError>('relay://error', event => handler(event.payload)),
};

export async function resolveUpstream(config: AppConfig): Promise<Hop> {
  return config.upstreamMode === 'system' ? api.detectSystemProxy() : config.upstream;
}
