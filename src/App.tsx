import { getCurrentWindow } from '@tauri-apps/api/window';
import { AlertTriangle, Minus, Settings, X } from 'lucide-react';
import { useEffect } from 'react';

import { HopSheet } from './components/HopSheet';
import { Logo } from './components/Logo';
import { RelayTrack } from './components/RelayTrack';
import { SettingsSheet } from './components/SettingsSheet';
import { TerminalSheet } from './components/TerminalSheet';
import { TrafficCard } from './components/TrafficCard';
import { Card, IconButton, ToggleRow } from './components/ui';
import { describeError, t } from './i18n';
import { useStore } from './store';

const isMac = navigator.userAgent.includes('Mac');

function WindowControls() {
  const appWindow = getCurrentWindow();
  return (
    <>
      <span className='mx-0.5 h-4 w-px bg-line' />
      <IconButton title={t.minimize} onClick={() => appWindow.minimize()}>
        <Minus className='size-[15px]' />
      </IconButton>
      <IconButton
        title={t.close}
        onClick={() => appWindow.close()}
        className='hover:bg-danger hover:text-white'
      >
        <X className='size-[15px]' />
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
      <span className='relative flex size-2'>
        {running && <span className='pulse-ring absolute inset-0 rounded-full bg-ok' />}
        <span className={`relative size-2 rounded-full ${running ? 'bg-ok' : 'bg-faint'}`} />
      </span>
      {running ? t.running : t.stopped}
    </div>
  );
}

function SystemProxyCard() {
  const checked = useStore(s => s.status?.config.useAsSystemProxy ?? false);
  const busy = useStore(s => s.busy);
  const setUseAsSystemProxy = useStore(s => s.setUseAsSystemProxy);
  return (
    <Card className='p-4'>
      <ToggleRow
        title={t.useAsSystemProxy}
        hint={t.useAsSystemProxyHint}
        checked={checked}
        disabled={busy}
        onChange={setUseAsSystemProxy}
      />
    </Card>
  );
}

function ErrorToast() {
  const error = useStore(s => s.error);
  const dismiss = useStore(s => s.dismissError);
  if (!error) {
    return null;
  }
  return (
    <div
      role='alert'
      className='toast-in fixed inset-x-[18px] bottom-[18px] z-20 flex items-start gap-2.5 rounded-xl border border-danger/25 bg-surface px-3.5 py-2.5 text-[12px] shadow-card'
    >
      <AlertTriangle className='mt-px size-4 shrink-0 text-danger' />
      <span className='flex-1 leading-snug'>{describeError(error)}</span>
      <button
        type='button'
        title={t.close}
        onClick={dismiss}
        className='text-faint hover:text-text'
      >
        <X className='size-4' />
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
    <div className='flex h-full flex-col'>
      <header
        data-tauri-drag-region
        className={`flex h-[50px] shrink-0 items-center justify-between pr-3 ${
          isMac ? 'pl-[90px]' : 'pl-[18px]'
        }`}
      >
        <div data-tauri-drag-region className='pointer-events-none flex items-center gap-1.5'>
          <Logo size={22} animated={running} />
          <span className='text-[13.5px] font-semibold tracking-tight'>Baton</span>
        </div>
        <div className='flex items-center gap-1.5'>
          <StatusPill />
          <IconButton title={t.settings} onClick={() => openSheet('settings')}>
            <Settings className='size-[15px]' />
          </IconButton>
          {!isMac && <WindowControls />}
        </div>
      </header>

      {ready && (
        <main className='flex-1 space-y-3 overflow-y-auto px-[18px] pb-[18px]'>
          <RelayTrack />
          <TrafficCard />
          <SystemProxyCard />
        </main>
      )}

      <ErrorToast />
      {ready && sheet === 'upstream' && <HopSheet role='upstream' />}
      {ready && sheet === 'landing' && <HopSheet role='landing' />}
      {ready && sheet === 'terminal' && <TerminalSheet />}
      {ready && sheet === 'settings' && <SettingsSheet />}
    </div>
  );
}
