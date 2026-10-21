import { Eye, EyeOff, Globe2, Loader2 } from 'lucide-react';
import { useEffect, useState } from 'react';

import {
  api,
  type CommandError,
  type Hop,
  type ProxyProtocol,
  resolveUpstream,
  toCommandError,
  type UpstreamMode,
} from '../api';
import { formatEndpoint, formatLocation, parsePort } from '../format';
import { t } from '../i18n';
import { type ProbeState, useStore } from '../store';
import { Sheet } from './Sheet';
import { Button, ErrorNote, Segmented, TextField } from './ui';

const MODES: {value: UpstreamMode; label: string;}[] = [{value: 'system', label: t.followSystem}, {
  value: 'manual',
  label: t.manual,
}];

const PROTOCOLS: {value: ProxyProtocol; label: string;}[] = [{value: 'socks5', label: 'SOCKS5'}, {
  value: 'http',
  label: 'HTTP',
}];

type Detection = {phase: 'running';} | {phase: 'done'; hop: Hop;} | {
  phase: 'failed';
  error: CommandError;
};

function DetectedProxy({detection}: {detection: Detection | null;}) {
  if (detection?.phase === 'failed') {
    return <ErrorNote error={detection.error} />;
  }
  const hop = detection?.phase === 'done' ? detection.hop : null;
  return (
    <div className='h-[52px] rounded-lg border border-line bg-surface-2 px-3 py-2'>
      <div className='text-[11px] leading-4 text-muted'>{t.detectedProxy}</div>
      {detection?.phase === 'running' && (
        <div className='mt-1 flex items-center gap-1.5 text-[12px] leading-5 text-muted'>
          <Loader2 className='size-3.5 animate-spin' />
          {t.detecting}
        </div>
      )}
      {hop && (
        <div className='mt-1 flex items-center gap-2 font-mono text-[12.5px] leading-5'>
          <span className='rounded bg-accent-soft px-1.5 text-[10.5px] font-semibold text-accent'>
            {hop.protocol === 'socks5' ? 'SOCKS5' : 'HTTP'}
          </span>
          {formatEndpoint(hop.host, hop.port)}
        </div>
      )}
    </div>
  );
}

function ProbeResult({probe}: {probe: ProbeState | null;}) {
  if (!probe || probe.phase === 'running') {
    return null;
  }
  if (probe.phase === 'failed') {
    return <ErrorNote error={probe.error} />;
  }
  const {info} = probe;
  return (
    <div className='flex items-center gap-2.5 rounded-lg bg-ok-soft px-3 py-2 text-[11.5px]'>
      <Globe2 className='size-4 shrink-0 text-ok' />
      <div className='min-w-0 flex-1'>
        <div className='font-mono text-[12.5px] font-semibold text-text'>{info.ip}</div>
        <div className='truncate text-muted' title={info.org}>
          {formatLocation([info.country, info.region, info.city, info.org])}
        </div>
      </div>
      <span className='shrink-0 font-mono text-muted'>{info.latencyMs} ms</span>
    </div>
  );
}

export function HopSheet({role}: {role: 'upstream' | 'landing';}) {
  const config = useStore(s => s.status!.config);
  const saveConfig = useStore(s => s.saveConfig);
  const isUpstream = role === 'upstream';
  const [hop, setHop] = useState<Hop>(config[role]);
  const [mode, setMode] = useState<UpstreamMode>(config.upstreamMode);
  const [detection, setDetection] = useState<Detection | null>(null);
  const [probe, setProbe] = useState<ProbeState | null>(null);
  const [revealPassword, setRevealPassword] = useState(false);
  const followSystem = isUpstream && mode === 'system';
  const update = (patch: Partial<Hop>) => {
    setHop(current => ({...current, ...patch}));
    setProbe(null);
  };
  const changeMode = (next: UpstreamMode) => {
    setMode(next);
    setProbe(null);
  };

  useEffect(() => {
    if (!followSystem) {
      return;
    }
    setDetection({phase: 'running'});
    api.detectSystemProxy().then(
      detected => setDetection({phase: 'done', hop: detected}),
      raw => setDetection({phase: 'failed', error: toCommandError(raw)}),
    );
  }, [followSystem]);

  const runProbe = async () => {
    setProbe({phase: 'running'});
    try {
      const hops = isUpstream
        ? [followSystem ? await api.detectSystemProxy() : hop]
        : [await resolveUpstream(config), hop];
      setProbe({phase: 'done', info: await api.probe(hops)});
    } catch (raw) {
      setProbe({phase: 'failed', error: toCommandError(raw)});
    }
  };

  const save = () =>
    saveConfig(
      isUpstream ? {...config, upstreamMode: mode, upstream: hop} : {...config, landing: hop},
    );

  const protocolPicker = (
    <Segmented
      value={hop.protocol}
      options={PROTOCOLS}
      onChange={protocol => update({protocol})}
    />
  );

  return (
    <Sheet
      title={isUpstream ? t.upstreamTitle : t.landingTitle}
      hint={isUpstream ? (followSystem ? t.upstreamSystemHint : t.upstreamHint) : t.landingHint}
      onSave={save}
    >
      <div className='flex items-center justify-between'>
        {isUpstream
          ? <Segmented value={mode} options={MODES} onChange={changeMode} />
          : protocolPicker}
        <Button
          size='sm'
          loading={probe?.phase === 'running'}
          disabled={probe?.phase === 'running'}
          title={isUpstream ? t.probeUpstreamHint : t.probeChainHint}
          onClick={runProbe}
        >
          {probe?.phase === 'running' ? t.probing : t.probe}
        </Button>
      </div>
      {followSystem ? <DetectedProxy detection={detection} /> : (
        <>
          {isUpstream && protocolPicker}
          <div className='flex gap-2'>
            <TextField
              label={t.host}
              className='flex-1'
              value={hop.host}
              autoFocus={!hop.host}
              placeholder={isUpstream ? '127.0.0.1' : 'proxy.example.com'}
              onChange={event => update({host: event.target.value.trim()})}
            />
            <TextField
              label={t.port}
              className='w-24'
              inputMode='numeric'
              value={hop.port || ''}
              placeholder={isUpstream ? '7897' : '8080'}
              onChange={event => update({port: parsePort(event.target.value)})}
            />
          </div>
          <div className='flex gap-2'>
            <TextField
              label={t.username}
              className='flex-1'
              value={hop.username}
              placeholder={t.optional}
              onChange={event => update({username: event.target.value})}
            />
            <TextField
              label={t.password}
              className='flex-1'
              type={revealPassword ? 'text' : 'password'}
              value={hop.password}
              placeholder={t.optional}
              onChange={event => update({password: event.target.value})}
              trailing={
                <button
                  type='button'
                  tabIndex={-1}
                  onClick={() => setRevealPassword(value => !value)}
                  className='px-2 text-faint hover:text-text'
                >
                  {revealPassword ? <EyeOff className='size-3.5' /> : <Eye className='size-3.5' />}
                </button>
              }
            />
          </div>
        </>
      )}
      <ProbeResult probe={probe} />
    </Sheet>
  );
}
