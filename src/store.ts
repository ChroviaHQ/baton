import { create } from 'zustand';

import {
  api,
  resolveUpstream,
  toCommandError,
  type AppConfig,
  type CommandError,
  type EgressInfo,
  type Stats,
  type Status,
} from './api';

const HISTORY = 60;

export type ProbeState =
  | { phase: 'running' }
  | { phase: 'done'; info: EgressInfo }
  | { phase: 'failed'; error: CommandError };

export type Sheet = 'upstream' | 'landing' | 'entry' | 'settings' | null;

type Sample = { up: number; down: number };

type Store = {
  status: Status | null;
  stats: Stats | null;
  history: Sample[];
  busy: boolean;
  error: CommandError | null;
  chainProbe: ProbeState | null;
  sheet: Sheet;
  init: () => Promise<() => void>;
  toggle: () => Promise<void>;
  saveConfig: (config: AppConfig) => Promise<CommandError | null>;
  setSystemProxy: (value: boolean) => Promise<void>;
  probeChain: () => Promise<void>;
  openSheet: (sheet: Sheet) => void;
  dismissError: () => void;
};

export const useStore = create<Store>((set, get) => {
  const run = async (action: () => Promise<Status>): Promise<CommandError | null> => {
    set({ busy: true });
    try {
      set({ status: await action(), error: null });
      return null;
    } catch (raw) {
      const error = toCommandError(raw);
      set({ status: await api.getStatus() });
      return error;
    } finally {
      set({ busy: false });
    }
  };

  return {
    status: null,
    stats: null,
    history: Array.from({ length: HISTORY }, () => ({ up: 0, down: 0 })),
    busy: false,
    error: null,
    chainProbe: null,
    sheet: null,

    init: async () => {
      const unlisten = await Promise.all([
        api.onStatus(status => set({ status })),
        api.onStats(stats => {
          const history = [...get().history.slice(1), { up: stats.upRate, down: stats.downRate }];
          set({ stats, history });
        }),
        api.onError(error => set({ error })),
      ]);
      set({ status: await api.getStatus() });
      return () => unlisten.forEach(fn => fn());
    },

    toggle: async () => {
      const { status } = get();
      if (!status) {
        return;
      }
      const error = await run(status.running ? api.stop : api.start);
      if (error) {
        set({ error });
      }
    },

    saveConfig: async config => {
      const error = await run(() => api.saveConfig(config));
      if (!error) {
        set({ chainProbe: null });
      }
      return error;
    },

    setSystemProxy: async value => {
      const { status } = get();
      if (!status) {
        return;
      }
      const error = await run(() => api.saveConfig({ ...status.config, systemProxy: value }));
      if (error) {
        set({ error });
      }
    },

    probeChain: async () => {
      const { status } = get();
      if (!status) {
        return;
      }
      set({ chainProbe: { phase: 'running' }, error: null });
      try {
        const upstream = await resolveUpstream(status.config);
        const info = await api.probe([upstream, status.config.landing]);
        set({ chainProbe: { phase: 'done', info } });
      } catch (raw) {
        const error = toCommandError(raw);
        set({ chainProbe: { phase: 'failed', error }, error });
      }
    },

    openSheet: sheet => set({ sheet }),

    dismissError: () => set({ error: null }),
  };
});
