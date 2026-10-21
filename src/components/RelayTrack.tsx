import { Globe, Laptop, Loader2, MapPin, Power, Server } from 'lucide-react';
import type { ComponentType } from 'react';

import type { AppConfig } from '../api';
import { formatEndpoint, formatLocation } from '../format';
import { t } from '../i18n';
import { useStore } from '../store';
import { Card } from './ui';

const LOOPBACK = new Set(['127.0.0.1', 'localhost', '::1']);

type TrackNodeProps = {
  icon: ComponentType<{className?: string;}>;
  label: string;
  value: string;
  active: boolean;
  loading?: boolean;
  failed?: boolean;
  title: string;
  onClick: () => void;
};

function TrackNode(
  {icon: Icon, label, value, active, loading, failed, title, onClick}: TrackNodeProps,
) {
  return (
    <button
      type='button'
      title={title}
      onClick={onClick}
      className='group relative z-10 flex min-w-0 flex-col items-center text-center'
    >
      <span className='rounded-full bg-surface'>
        <span
          className={`grid size-10 place-items-center rounded-full border transition group-hover:ring-4 group-hover:ring-accent-soft ${
            failed
              ? 'border-danger/40 bg-danger-soft text-danger'
              : active
              ? 'border-accent/30 bg-accent-soft text-accent'
              : 'border-line bg-surface-2 text-muted group-hover:text-text'
          }`}
        >
          {loading
            ? <Loader2 className='size-[18px] animate-spin' />
            : <Icon className='size-[18px]' />}
        </span>
      </span>
      <span className='mt-2 text-[11px] leading-4 font-medium text-muted'>{label}</span>
      <span
        className={`mt-0.5 w-full truncate px-0.5 text-[11px] leading-4 font-medium tabular-nums ${
          value ? 'text-text' : 'text-faint'
        }`}
      >
        {value || t.notSet}
      </span>
    </button>
  );
}

function upstreamLabel(config: AppConfig): string {
  if (config.upstreamMode === 'system') {
    return t.followSystemShort;
  }
  const {host, port} = config.upstream;
  if (!host) {
    return '';
  }
  return LOOPBACK.has(host) ? `:${port}` : formatEndpoint(host, port);
}

function TrackLine({active}: {active: boolean;}) {
  return (
    <svg className='absolute top-5 left-[12.5%] h-px w-3/4 overflow-visible'>
      <line
        x1='0'
        y1='0'
        x2='100%'
        y2='0'
        strokeWidth='2'
        strokeLinecap='round'
        className={active ? 'relay-flow stroke-accent' : 'stroke-line'}
      />
    </svg>
  );
}

export function RelayTrack() {
  const status = useStore(s => s.status);
  const busy = useStore(s => s.busy);
  const probe = useStore(s => s.chainProbe);
  const toggleRelay = useStore(s => s.toggleRelay);
  const openSheet = useStore(s => s.openSheet);
  const probeChain = useStore(s => s.probeChain);
  if (!status) {
    return null;
  }

  const {running, config} = status;
  const egress = probe?.phase === 'done'
    ? probe.info.ip
    : probe?.phase === 'failed'
    ? t.probeFailed
    : t.egressPrompt;

  return (
    <Card className='p-4'>
      <div className='relative grid grid-cols-4'>
        <TrackLine active={running} />
        <TrackNode
          icon={Laptop}
          label={t.local}
          value={`:${config.listenPort}`}
          active={running}
          title={t.terminalTitle}
          onClick={() => openSheet('terminal')}
        />
        <TrackNode
          icon={Server}
          label={t.upstream}
          value={upstreamLabel(config)}
          active={running}
          title={t.editUpstream}
          onClick={() => openSheet('upstream')}
        />
        <TrackNode
          icon={MapPin}
          label={t.landing}
          value={config.landing.host}
          active={running}
          title={t.editLanding}
          onClick={() => openSheet('landing')}
        />
        <TrackNode
          icon={Globe}
          label={t.egress}
          value={egress}
          active={running}
          loading={probe?.phase === 'running'}
          failed={probe?.phase === 'failed'}
          title={probe?.phase === 'done'
            ? formatLocation([probe.info.ip, probe.info.country, probe.info.city, probe.info.org])
            : t.probeChainHint}
          onClick={probeChain}
        />
      </div>

      <button
        type='button'
        onClick={toggleRelay}
        disabled={busy}
        className={`mt-4 flex h-11 w-full items-center justify-center gap-2 rounded-xl text-[13.5px] font-semibold transition active:scale-[0.99] disabled:opacity-70 ${
          running
            ? 'border border-line bg-surface-2 text-text hover:border-danger/40 hover:text-danger'
            : 'bg-accent text-on-accent hover:brightness-110'
        }`}
      >
        {busy ? <Loader2 className='size-4 animate-spin' /> : <Power className='size-4' />}
        {busy ? t.working : running ? t.stop : t.start}
      </button>
    </Card>
  );
}
