import { useState } from 'react';

import { t } from '../i18n';
import { useStore } from '../store';
import { Sheet } from './Sheet';
import { CopyIcon, Segmented, useCopy } from './ui';

type Shell = 'posix' | 'fish' | 'powershell' | 'cmd';

const SHELLS: {value: Shell; label: string;}[] = navigator.userAgent.includes('Windows')
  ? [{value: 'powershell', label: 'PowerShell'}, {value: 'cmd', label: 'CMD'}]
  : [{value: 'posix', label: 'bash / zsh'}, {value: 'fish', label: 'fish'}];

const NO_PROXY = 'localhost,127.0.0.1,::1';

function proxyCommand(shell: Shell, port: number): string {
  const proxy = `http://127.0.0.1:${port}`;
  switch (shell) {
    case 'posix':
      return `export http_proxy=${proxy} https_proxy=${proxy} all_proxy=${proxy} no_proxy=${NO_PROXY}`;
    case 'fish':
      return `set -gx http_proxy ${proxy}; set -gx https_proxy ${proxy}; set -gx all_proxy ${proxy}; set -gx no_proxy ${NO_PROXY}`;
    case 'powershell':
      return `$env:http_proxy="${proxy}"; $env:https_proxy="${proxy}"; $env:all_proxy="${proxy}"; $env:no_proxy="${NO_PROXY}"`;
    case 'cmd':
      return `set "http_proxy=${proxy}" & set "https_proxy=${proxy}" & set "all_proxy=${proxy}" & set "no_proxy=${NO_PROXY}"`;
  }
}

export function TerminalSheet() {
  const port = useStore(s => s.status!.config.listenPort);
  const [shell, setShell] = useState<Shell>(SHELLS[0].value);
  const [copied, copy] = useCopy();
  const command = proxyCommand(shell, port);

  return (
    <Sheet title={t.terminalTitle} hint={t.terminalHint}>
      <Segmented value={shell} options={SHELLS} onChange={setShell} />
      <button
        type='button'
        onClick={() => copy(command)}
        title={copied ? t.copied : t.copy}
        className='group flex w-full items-start gap-2 rounded-lg border border-line bg-surface-2 px-3 py-2.5 text-left transition hover:border-accent/40'
      >
        <code className='flex-1 font-mono text-[11.5px] leading-relaxed break-all text-text'>
          {command}
        </code>
        <span className='mt-0.5'>
          <CopyIcon copied={copied} />
        </span>
      </button>
    </Sheet>
  );
}
