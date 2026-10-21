import { Check, Copy } from 'lucide-react';
import { useState } from 'react';

import { t } from '../i18n';
import { useStore } from '../store';
import { Sheet } from './Sheet';
import { Segmented } from './ui';

type Shell = 'posix' | 'fish' | 'powershell' | 'cmd';

const isWindows = navigator.userAgent.includes('Windows');

const SHELLS: { value: Shell; label: string }[] = isWindows
  ? [
      { value: 'powershell', label: 'PowerShell' },
      { value: 'cmd', label: 'CMD' },
    ]
  : [
      { value: 'posix', label: 'bash / zsh' },
      { value: 'fish', label: 'fish' },
    ];

const NO_PROXY = 'localhost,127.0.0.1,::1';

function command(shell: Shell, port: number): string {
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

function useCopy(): [boolean, (value: string) => Promise<void>] {
  const [copied, setCopied] = useState(false);
  const copy = async (value: string) => {
    await navigator.clipboard.writeText(value);
    setCopied(true);
    setTimeout(() => setCopied(false), 1200);
  };
  return [copied, copy];
}

function CopyIcon({ copied }: { copied: boolean }) {
  return copied ? (
    <Check className="size-3.5 shrink-0 text-ok" />
  ) : (
    <Copy className="size-3.5 shrink-0 text-faint group-hover:text-accent" />
  );
}

function CommandBlock({ value }: { value: string }) {
  const [copied, copy] = useCopy();
  return (
    <button
      type="button"
      onClick={() => copy(value)}
      title={copied ? t.copied : t.copy}
      className="group flex w-full items-start gap-2 rounded-lg border border-line bg-surface-2 px-3 py-2.5 text-left transition hover:border-accent/40"
    >
      <code className="flex-1 font-mono text-[11.5px] leading-relaxed break-all text-text">
        {value}
      </code>
      <span className="mt-0.5">
        <CopyIcon copied={copied} />
      </span>
    </button>
  );
}

export function EntrySheet() {
  const port = useStore(s => s.status!.config.listenPort);
  const openSheet = useStore(s => s.openSheet);
  const [shell, setShell] = useState<Shell>(SHELLS[0].value);

  return (
    <Sheet title={t.editEntry} hint={t.entryHint} onClose={() => openSheet(null)}>
      <Segmented value={shell} options={SHELLS} onChange={setShell} />
      <CommandBlock value={command(shell, port)} />
    </Sheet>
  );
}
