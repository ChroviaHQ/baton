import { useState } from 'react';

import { t } from '../i18n';
import { isWindows } from '../platform';
import { useStore } from '../store';
import { Sheet } from './Sheet';
import { CopyButton, Segmented } from './ui';

type Shell = 'posix' | 'fish' | 'powershell' | 'cmd';

const SHELLS: {value: Shell; label: string;}[] = isWindows
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
  const command = proxyCommand(shell, port);

  return (
    <Sheet title={t.terminalTitle} hint={t.terminalHint}>
      <Segmented value={shell} options={SHELLS} onChange={setShell} />
      <CopyButton value={command}>
        <code className='flex-1 font-mono text-[11.5px] leading-relaxed break-all text-text'>
          {command}
        </code>
      </CopyButton>
    </Sheet>
  );
}
