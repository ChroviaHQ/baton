import { create } from 'zustand';

import {
  api,
  type AppConfig,
  type CommandError,
  type EgressInfo,
  resolveUpstream,
  type Stats,
  type Status,
  toCommandError,
} from './api';

const HISTORY_LENGTH = 60;

export type ProbeState = {phase: 'running';} | {phase: 'done'; info: EgressInfo;} | {
  phase: 'failed';
  error: CommandError;
};

type SheetId = 'upstream' | 'landing' | 'terminal' | 'settings';

const SHEET_ERRORS: Record<string, SheetId> = {
  invalid_listen_port: 'settings',
  port_in_use: 'settings',
  bind_failed: 'settings',
  system_proxy_not_set: 'upstream',
  system_proxy_is_baton: 'upstream',
  system_proxy_read_failed: 'upstream',
};

const HOP_ERRORS = new Set(['empty_host', 'invalid_port', 'credentials_too_long']);

function sheetFor(error: CommandError): SheetId | null {
  if (HOP_ERRORS.has(error.code)) {
    return error.hop === 0 ? 'upstream' : 'landing';
  }
  return SHEET_ERRORS[error.code] ?? null;
}

type RateSample = {up: number; down: number;};

type Store = {
  status: Status | null;
  stats: Stats | null;
  rateHistory: RateSample[];
  busy: boolean;
  error: CommandError | null;
  chainProbe: ProbeState | null;
  sheet: SheetId | null;
  sheetError: CommandError | null;
  init: () => Promise<() => void>;
  toggleRelay: () => Promise<void>;
  saveConfig: (config: AppConfig) => Promise<CommandError | null>;
  setUseAsSystemProxy: (value: boolean) => Promise<void>;
  probeChain: () => Promise<void>;
  openSheet: (sheet: SheetId) => void;
  closeSheet: () => void;
  setSheetError: (error: CommandError | null) => void;
  dismissError: () => void;
};

export const useStore = create<Store>((set, get) => {
  const report = (error: CommandError) => {
    const sheet = sheetFor(error);
    set(sheet ? {sheet, sheetError: error, error: null} : {error});
  };

  const runCommand = async (command: () => Promise<Status>): Promise<CommandError | null> => {
    set({busy: true});
    try {
      set({status: await command(), error: null});
      return null;
    } catch (raw) {
      const error = toCommandError(raw);
      try {
        set({status: await api.getStatus()});
      } catch {}
      return error;
    } finally {
      set({busy: false});
    }
  };

  return {
    status: null,
    stats: null,
    rateHistory: Array.from({length: HISTORY_LENGTH}, () => ({up: 0, down: 0})),
    busy: false,
    error: null,
    chainProbe: null,
    sheet: null,
    sheetError: null,

    init: async () => {
      const unlisten = await Promise.all([
        api.onStatus(status => set({status})),
        api.onStats(stats => {
          const rateHistory = [...get().rateHistory.slice(1), {
            up: stats.upRate,
            down: stats.downRate,
          }];
          set({stats, rateHistory});
        }),
        api.onError(error => {
          report(error);
          void api.takePendingError();
        }),
      ]);
      const [status, pending] = await Promise.all([api.getStatus(), api.takePendingError()]);
      set({status});
      if (pending) {
        report(pending);
      }
      return () => unlisten.forEach(fn => fn());
    },

    toggleRelay: async () => {
      const {status} = get();
      if (!status) {
        return;
      }
      const error = await runCommand(status.running ? api.stopRelay : api.startRelay);
      if (error) {
        report(error);
      }
    },

    saveConfig: async config => {
      const error = await runCommand(() => api.saveConfig(config));
      if (!error) {
        set({chainProbe: null});
      }
      return error;
    },

    setUseAsSystemProxy: async value => {
      const {status} = get();
      if (!status) {
        return;
      }
      const error = await runCommand(() =>
        api.saveConfig({...status.config, useAsSystemProxy: value})
      );
      if (error) {
        report(error);
      }
    },

    probeChain: async () => {
      const {status} = get();
      if (!status) {
        return;
      }
      set({chainProbe: {phase: 'running'}, error: null});
      try {
        const upstream = await resolveUpstream(status.config);
        const info = await api.probe([upstream, status.config.landing]);
        set({chainProbe: {phase: 'done', info}});
      } catch (raw) {
        const error = toCommandError(raw);
        set({chainProbe: {phase: 'failed', error}});
        report(error);
      }
    },

    openSheet: sheet => set({sheet, sheetError: null, error: null}),

    closeSheet: () => set({sheet: null, sheetError: null}),

    setSheetError: sheetError => set({sheetError}),

    dismissError: () => set({error: null}),
  };
});
