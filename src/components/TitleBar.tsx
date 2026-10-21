import { getCurrentWindow } from '@tauri-apps/api/window';
import { ArrowUpCircle, Loader2, Minus, Settings, X } from 'lucide-react';
import { useState } from 'react';

import { format, t } from '../i18n';
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

const PILL = 'flex h-6.5 items-center gap-1.5 rounded-md px-2 text-[11.5px] font-medium';

function StatusPill({running}: {running: boolean;}) {
  return (
    <div className={`${PILL} ${running ? 'bg-ok-soft text-ok' : 'bg-surface-2 text-muted'}`}>
      <span className='relative flex size-2'>
        {running && <span className='pulse-ring absolute inset-0 rounded-full bg-ok' />}
        <span className={`relative size-2 rounded-full ${running ? 'bg-ok' : 'bg-faint'}`} />
      </span>
      {running ? t.running : t.stopped}
    </div>
  );
}

function UpdateButton({version}: {version: string;}) {
  const installUpdate = useStore(s => s.installUpdate);
  const [updating, setUpdating] = useState(false);
  const install = async () => {
    setUpdating(true);
    await installUpdate();
    setUpdating(false);
  };
  return (
    <button
      type='button'
      title={t.updateHint}
      disabled={updating}
      onClick={install}
      className={`${PILL} bg-accent-soft text-accent transition hover:brightness-110 disabled:opacity-60`}
    >
      {updating
        ? <Loader2 className='size-3.5 animate-spin' />
        : <ArrowUpCircle className='size-3.5' />}
      {updating ? t.updating : format(t.updateTo, {version})}
    </button>
  );
}

export function TitleBar() {
  const running = useStore(s => s.status?.running ?? false);
  const openSheet = useStore(s => s.openSheet);
  const update = useStore(s => s.update);
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
        {update ? <UpdateButton version={update} /> : <StatusPill running={running} />}
        <IconButton title={t.settings} onClick={() => openSheet('settings')}>
          <Settings className='size-3.75' />
        </IconButton>
        {!isMac && <WindowControls />}
      </div>
    </header>
  );
}
