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

export type AsyncState<T> = {status: 'pending';} | {status: 'succeeded'; value: T;} | {
  status: 'failed';
  error: CommandError;
};

export type HopRole = 'upstream' | 'landing';

export type SheetId = HopRole | 'terminal' | 'settings';

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
  chainProbe: AsyncState<EgressInfo> | null;
  sheet: SheetId | null;
  sheetError: CommandError | null;
  update: string | null;
  init: () => Promise<() => void>;
  installUpdate: () => Promise<void>;
  toggleRelay: () => Promise<void>;
  saveConfig: (patch: Partial<AppConfig>) => Promise<CommandError | null>;
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
      return toCommandError(raw);
    } finally {
      set({busy: false});
    }
  };

  const updateConfig = (patch: Partial<AppConfig>) =>
    runCommand(() => api.saveConfig({...get().status!.config, ...patch}));

  return {
    status: null,
    stats: null,
    rateHistory: Array.from({length: HISTORY_LENGTH}, () => ({up: 0, down: 0})),
    busy: false,
    error: null,
    chainProbe: null,
    sheet: null,
    sheetError: null,
    update: null,

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
        api.onUpdate(update => set({update})),
        api.onError(error => {
          report(error);
          void api.takePendingError();
        }),
      ]);
      const [status, pending, update] = await Promise.all([
        api.getStatus(),
        api.takePendingError(),
        api.pendingUpdate(),
      ]);
      set({status, update});
      if (pending) {
        report(pending);
      }
      return () => unlisten.forEach(fn => fn());
    },

    installUpdate: async () => {
      try {
        await api.installUpdate();
      } catch (raw) {
        report(toCommandError(raw));
      }
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

    saveConfig: async patch => {
      const error = await updateConfig(patch);
      if (!error) {
        set({chainProbe: null});
      }
      return error;
    },

    setUseAsSystemProxy: async value => {
      const error = await updateConfig({useAsSystemProxy: value});
      if (error) {
        report(error);
      }
    },

    probeChain: async () => {
      const {status} = get();
      if (!status) {
        return;
      }
      set({chainProbe: {status: 'pending'}, error: null});
      try {
        const upstream = await resolveUpstream(status.config);
        const info = await api.probe([upstream, status.config.landing]);
        set({chainProbe: {status: 'succeeded', value: info}});
      } catch (raw) {
        const error = toCommandError(raw);
        set({chainProbe: {status: 'failed', error}});
        report(error);
      }
    },

    openSheet: sheet => set({sheet, sheetError: null, error: null}),

    closeSheet: () => set({sheet: null, sheetError: null}),

    setSheetError: sheetError => set({sheetError}),

    dismissError: () => set({error: null}),
  };
});
