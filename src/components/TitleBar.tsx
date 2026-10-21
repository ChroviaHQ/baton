import { getCurrentWindow } from '@tauri-apps/api/window';
import { Minus, Settings, X } from 'lucide-react';

import { t } from '../i18n';
import { isMac } from '../platform';
import { useStore } from '../store';
import { Logo } from './Logo';
import { IconButton } from './ui';

function WindowControls() {
  const appWindow = getCurrentWindow();
  return (
    <>
      <span className='mx-0.5 h-4 w-px bg-line' />
      <IconButton title={t.minimize} onClick={() => appWindow.minimize()}>
        <Minus className='size-3.75' />
      </IconButton>
      <IconButton
        title={t.close}
        onClick={() => appWindow.close()}
        className='hover:bg-danger hover:text-white'
      >
        <X className='size-3.75' />
      </IconButton>
    </>
  );
}

function StatusPill({running}: {running: boolean;}) {
  return (
    <div
      className={`flex h-6.5 items-center gap-1.5 rounded-md px-2 text-[11.5px] font-medium ${
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

export function TitleBar() {
  const running = useStore(s => s.status?.running ?? false);
  const openSheet = useStore(s => s.openSheet);
  return (
    <header
      data-tauri-drag-region
      className={`flex h-12.5 shrink-0 items-center justify-between pr-3 ${
        isMac ? 'pl-22.5' : 'pl-4.5'
      }`}
    >
      <div data-tauri-drag-region className='pointer-events-none flex items-center gap-1.5'>
        <Logo size={22} animated={running} />
        <span className='text-[13.5px] font-semibold tracking-tight'>Baton</span>
      </div>
      <div className='flex items-center gap-1.5'>
        <StatusPill running={running} />
        <IconButton title={t.settings} onClick={() => openSheet('settings')}>
          <Settings className='size-3.75' />
        </IconButton>
        {!isMac && <WindowControls />}
      </div>
    </header>
  );
}
