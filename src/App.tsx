import { getCurrentWindow } from '@tauri-apps/api/window';
import { AlertTriangle, Minus, Settings, X } from 'lucide-react';
import { useEffect } from 'react';

import { EntrySheet } from './components/EntrySheet';
import { HopSheet } from './components/HopSheet';
import { Logo } from './components/Logo';
import { RelayTrack } from './components/RelayTrack';
import { SettingsSheet } from './components/SettingsSheet';
import { TrafficCard } from './components/TrafficCard';
import { Card, IconButton, Switch } from './components/ui';
import { describeError, t } from './i18n';
import { useStore } from './store';

const isMac = navigator.userAgent.includes('Mac');

function WindowControls() {
  const window = getCurrentWindow();
  return (
    <>
      <span className="mx-0.5 h-4 w-px bg-line" />
      <IconButton title={t.minimize} onClick={() => window.minimize()}>
        <Minus className="size-[15px]" />
      </IconButton>
      <IconButton
        title={t.close}
        onClick={() => window.close()}
        className="hover:bg-danger hover:text-white"
      >
        <X className="size-[15px]" />
      </IconButton>
    </>
  );
}

function StatusPill() {
  const running = useStore(s => s.status?.running ?? false);
  return (
    <div
      className={`flex h-[26px] items-center gap-1.5 rounded-md px-2 text-[11.5px] font-medium ${
        running ? 'bg-ok-soft text-ok' : 'bg-surface-2 text-muted'
      }`}
    >
      <span className="relative flex size-2">
        {running && <span className="pulse-ring absolute inset-0 rounded-full bg-ok" />}
        <span className={`relative size-2 rounded-full ${running ? 'bg-ok' : 'bg-faint'}`} />
      </span>
      {running ? t.running : t.stopped}
    </div>
  );
}

function SystemProxyRow() {
  const checked = useStore(s => s.status?.config.systemProxy ?? false);
  const busy = useStore(s => s.busy);
  const setSystemProxy = useStore(s => s.setSystemProxy);
  return (
    <Card className="flex items-center justify-between gap-4 p-4">
      <div className="min-w-0">
        <div className="text-[12.5px] leading-5 font-medium">{t.systemProxy}</div>
        <div className="mt-0.5 text-[11px] leading-4 text-muted">{t.systemProxyHint}</div>
      </div>
      <Switch checked={checked} disabled={busy} onChange={setSystemProxy} />
    </Card>
  );
}

function ErrorToast() {
  const error = useStore(s => s.error);
  const dismiss = useStore(s => s.dismissError);
  useEffect(() => {
    if (!error) {
      return;
    }
    const timer = setTimeout(dismiss, 8000);
    return () => clearTimeout(timer);
  }, [error, dismiss]);
  if (!error) {
    return null;
  }
  return (
    <div className="toast-in fixed inset-x-[18px] bottom-[18px] flex items-start gap-2.5 rounded-xl border border-danger/25 bg-surface px-3.5 py-2.5 text-[12px] shadow-card">
      <AlertTriangle className="mt-px size-4 shrink-0 text-danger" />
      <span className="flex-1 leading-snug">{describeError(error)}</span>
      <button type="button" onClick={dismiss} className="text-faint hover:text-text">
        <X className="size-4" />
      </button>
    </div>
  );
}

export function App() {
  const init = useStore(s => s.init);
  const ready = useStore(s => !!s.status);
  const sheet = useStore(s => s.sheet);
  const openSheet = useStore(s => s.openSheet);
  const running = useStore(s => s.status?.running ?? false);

  useEffect(() => {
    let cleanup: (() => void) | undefined;
    let cancelled = false;
    init().then(fn => {
      if (cancelled) {
        fn();
      } else {
        cleanup = fn;
      }
    });
    return () => {
      cancelled = true;
      cleanup?.();
    };
  }, [init]);

  return (
    <div className="flex h-full flex-col">
      <header
        data-tauri-drag-region
        className={`flex h-[50px] shrink-0 items-center justify-between pr-3 ${isMac ? 'pl-[90px]' : 'pl-[18px]'}`}
      >
        <div data-tauri-drag-region className="pointer-events-none flex items-center gap-1.5">
          <Logo size={22} animated={running} />
          <span className="text-[13.5px] font-semibold tracking-tight">Baton</span>
        </div>
        <div className="flex items-center gap-1.5">
          <StatusPill />
          <IconButton title={t.settings} onClick={() => openSheet('settings')}>
            <Settings className="size-[15px]" />
          </IconButton>
          {!isMac && <WindowControls />}
        </div>
      </header>

      {ready && (
        <main className="flex-1 space-y-3 overflow-y-auto px-[18px] pb-[18px]">
          <RelayTrack />
          <TrafficCard />
          <SystemProxyRow />
        </main>
      )}

      <ErrorToast />
      {ready && sheet === 'upstream' && <HopSheet which="upstream" />}
      {ready && sheet === 'landing' && <HopSheet which="landing" />}
      {ready && sheet === 'entry' && <EntrySheet />}
      {ready && sheet === 'settings' && <SettingsSheet />}
    </div>
  );
}
