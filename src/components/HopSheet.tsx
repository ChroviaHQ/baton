import { Eye, EyeOff, Globe2, Loader2 } from 'lucide-react';
import { useEffect, useState } from 'react';

import {
  api,
  type EgressInfo,
  type Hop,
  type ProxyProtocol,
  resolveUpstream,
  toCommandError,
  type UpstreamMode,
} from '../api';
import { formatEndpoint, formatLocation, parsePort } from '../format';
import { interpolate, t } from '../i18n';
import { type AsyncState, type HopRole, useStore } from '../store';
import { Sheet } from './Sheet';
import { Button, ErrorNote, IconButton, Segmented, TextField } from './ui';

const PROTOCOL_LABELS: Record<ProxyProtocol, string> = {socks5: 'SOCKS5', http: 'HTTP'};

const PROTOCOLS = (Object.keys(PROTOCOL_LABELS) as ProxyProtocol[]).map(value => ({
  value,
  label: PROTOCOL_LABELS[value],
}));

const MODES: {value: UpstreamMode; label: string;}[] = [{value: 'system', label: t.followSystem}, {
  value: 'manual',
  label: t.manual,
}];

const PLACEHOLDERS: Record<HopRole, {host: string; port: string;}> = {
  upstream: {host: '127.0.0.1', port: '7897'},
  landing: {host: 'proxy.example.com', port: '8080'},
};

function DetectedProxy({detection}: {detection: AsyncState<Hop> | null;}) {
  if (detection?.status === 'failed') {
    return <ErrorNote error={detection.error} />;
  }
  return (
    <div className='flex flex-col gap-1 rounded-lg border border-line bg-surface-2 px-3 py-2'>
      <div className='text-[11px] leading-4 text-muted'>{t.detectedProxy}</div>
      <div
        className={`flex h-lh items-center gap-2 text-[12px] leading-5 ${
          detection?.status === 'pending' ? 'text-muted' : ''
        }`}
      >
        {detection?.status === 'pending' && (
          <>
            <Loader2 className='size-3.5 animate-spin' />
            {t.detecting}
          </>
        )}
        {detection?.status === 'succeeded' && (
          <>
            <span className='rounded bg-accent-soft px-1.5 text-[10.5px] font-semibold text-accent'>
              {PROTOCOL_LABELS[detection.value.protocol]}
            </span>
            <span className='font-mono text-[12.5px]'>
              {formatEndpoint(detection.value.host, detection.value.port)}
            </span>
          </>
        )}
      </div>
    </div>
  );
}

function ProbeResult({probe}: {probe: AsyncState<EgressInfo> | null;}) {
  if (!probe || probe.status === 'pending') {
    return null;
  }
  if (probe.status === 'failed') {
    return <ErrorNote error={probe.error} />;
  }
  const info = probe.value;
  return (
    <div className='flex items-center gap-2.5 rounded-lg bg-ok-soft px-3 py-2 text-[11.5px]'>
      <Globe2 className='size-4 shrink-0 text-ok' />
      <div className='min-w-0 flex-1'>
        <div className='font-mono text-[12.5px] font-semibold text-text'>{info.ip}</div>
        <div className='truncate text-muted' title={info.org}>
          {formatLocation([info.country, info.region, info.city, info.org])}
        </div>
      </div>
      <span className='shrink-0 font-mono text-muted'>
        {interpolate(t.latency, {value: info.latencyMs})}
      </span>
    </div>
  );
}

function HopFields(
  {role, hop, onChange}: {role: HopRole; hop: Hop; onChange: (patch: Partial<Hop>) => void;},
) {
  const [revealPassword, setRevealPassword] = useState(false);
  const placeholder = PLACEHOLDERS[role];
  return (
    <>
      <div className='flex gap-2'>
        <TextField
          label={t.host}
          className='flex-1'
          value={hop.host}
          autoFocus={!hop.host}
          placeholder={placeholder.host}
          onChange={event => onChange({host: event.target.value.trim()})}
        />
        <TextField
          label={t.port}
          className='w-24'
          inputMode='numeric'
          value={hop.port || ''}
          placeholder={placeholder.port}
          onChange={event => onChange({port: parsePort(event.target.value)})}
        />
      </div>
      <div className='flex gap-2'>
        <TextField
          label={t.username}
          className='flex-1'
          value={hop.username}
          placeholder={t.optional}
          onChange={event => onChange({username: event.target.value})}
        />
        <TextField
          label={t.password}
          className='flex-1'
          type={revealPassword ? 'text' : 'password'}
          value={hop.password}
          placeholder={t.optional}
          onChange={event => onChange({password: event.target.value})}
          trailing={
            <IconButton
              tabIndex={-1}
              title={revealPassword ? t.hidePassword : t.showPassword}
              onClick={() => setRevealPassword(value => !value)}
              className='mr-0.5'
            >
              {revealPassword ? <EyeOff className='size-3.5' /> : <Eye className='size-3.5' />}
            </IconButton>
          }
        />
      </div>
    </>
  );
}

export function HopSheet({role}: {role: HopRole;}) {
  const config = useStore(s => s.status!.config);
  const saveConfig = useStore(s => s.saveConfig);
  const isUpstream = role === 'upstream';
  const [hop, setHop] = useState<Hop>(config[role]);
  const [mode, setMode] = useState<UpstreamMode>(config.upstreamMode);
  const [detection, setDetection] = useState<AsyncState<Hop> | null>(null);
  const [probe, setProbe] = useState<AsyncState<EgressInfo> | null>(null);
  const followSystem = isUpstream && mode === 'system';
  const probing = probe?.status === 'pending';

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
    setDetection({status: 'pending'});
    api.detectSystemProxy().then(
      value => setDetection({status: 'succeeded', value}),
      raw => setDetection({status: 'failed', error: toCommandError(raw)}),
    );
  }, [followSystem]);

  const runProbe = async () => {
    setProbe({status: 'pending'});
    try {
      const hops = isUpstream
        ? [await resolveUpstream({...config, upstreamMode: mode, upstream: hop})]
        : [await resolveUpstream(config), hop];
      setProbe({status: 'succeeded', value: await api.probeEgress(hops)});
    } catch (raw) {
      setProbe({status: 'failed', error: toCommandError(raw)});
    }
  };

  const save = () => saveConfig(isUpstream ? {upstreamMode: mode, upstream: hop} : {landing: hop});

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
          loading={probing}
          disabled={probing}
          title={isUpstream ? t.probeUpstreamHint : t.probeChainHint}
          onClick={runProbe}
        >
          {probing ? t.probing : t.probe}
        </Button>
      </div>
      {followSystem ? <DetectedProxy detection={detection} /> : (
        <>
          {isUpstream && protocolPicker}
          <HopFields role={role} hop={hop} onChange={update} />
        </>
      )}
      <ProbeResult probe={probe} />
    </Sheet>
  );
}
