import { create } from 'zustand';

import {
  api,
  type AppConfig,
  type CommandError,
  type EgressInfo,
  type HopRole,
  type Stats,
  type Status,
  toCommandError,
} from './api';

const HISTORY_LENGTH = 60;

export type AsyncState<T> = {status: 'pending';} | {status: 'succeeded'; value: T;} | {
  status: 'failed';
  error: CommandError;
};

export function settle<T>(
  promise: Promise<T>,
): Promise<Exclude<AsyncState<T>, {status: 'pending';}>> {
  return promise.then(
    value => ({status: 'succeeded', value}),
    raw => ({status: 'failed', error: toCommandError(raw)}),
  );
}

export type SheetId = HopRole | 'local' | 'settings';

const SHEET_ERRORS: Record<string, SheetId> = {
  invalid_listen_port: 'local',
  port_in_use: 'local',
  bind_failed: 'local',
  invalid_bypass: 'settings',
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
  egressProbe: AsyncState<EgressInfo> | null;
  sheet: SheetId | null;
  sheetError: CommandError | null;
  updateVersion: string | null;
  init: () => Promise<() => void>;
  installUpdate: () => Promise<void>;
  toggleRelay: () => Promise<void>;
  saveConfig: (patch: Partial<AppConfig>) => Promise<CommandError | null>;
  setUseAsSystemProxy: (value: boolean) => Promise<void>;
  probeEgress: () => Promise<void>;
  openSheet: (sheet: SheetId) => void;
  closeSheet: () => void;
  setSheetError: (error: CommandError | null) => void;
  dismissError: () => void;
};

export const useStore = create<Store>((set, get) => {
  const receiveStatus = (status: Status) => {
    set({status, egressProbe: null});
  };

  const report = (error: CommandError) => {
    const sheet = sheetFor(error);
    set(sheet ? {sheet, sheetError: error, error: null} : {error});
  };

  const runCommand = async (command: () => Promise<Status>): Promise<CommandError | null> => {
    set({busy: true});
    try {
      receiveStatus(await command());
      set({error: null});
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
    egressProbe: null,
    sheet: null,
    sheetError: null,
    updateVersion: null,

    init: async () => {
      const unlisten = await Promise.all([
        api.onStatus(receiveStatus),
        api.onStats(stats => {
          const rateHistory = [...get().rateHistory.slice(1), {
            up: stats.upRate,
            down: stats.downRate,
          }];
          set({stats, rateHistory});
        }),
        api.onUpdate(updateVersion => set({updateVersion})),
        api.onError(error => {
          report(error);
          void api.takePendingError();
        }),
      ]);
      const [status, pending, updateVersion] = await Promise.all([
        api.getStatus(),
        api.takePendingError(),
        api.getPendingUpdate(),
      ]);
      receiveStatus(status);
      set({updateVersion});
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

    saveConfig: updateConfig,

    setUseAsSystemProxy: async value => {
      const error = await updateConfig({useAsSystemProxy: value});
      if (error) {
        report(error);
      }
    },

    probeEgress: async () => {
      const {status, egressProbe: current} = get();
      if (!status || current?.status === 'pending') {
        return;
      }
      const pending: AsyncState<EgressInfo> = {status: 'pending'};
      set({egressProbe: pending, error: null});
      const egressProbe = await settle(api.probeEgress(status.config, 'landing'));
      if (get().egressProbe !== pending) {
        return;
      }
      set({egressProbe});
      if (egressProbe.status === 'failed') {
        report(egressProbe.error);
      }
    },

    openSheet: sheet => set({sheet, sheetError: null, error: null}),

    closeSheet: () => set({sheet: null, sheetError: null}),

    setSheetError: sheetError => set({sheetError}),

    dismissError: () => set({error: null}),
  };
});
