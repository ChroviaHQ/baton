import { AlertTriangle, Eye, EyeOff, Globe2 } from 'lucide-react';
import { useEffect, useState } from 'react';

import {
  api,
  resolveUpstream,
  toCommandError,
  type CommandError,
  type Hop,
  type ProxyKind,
  type UpstreamMode,
} from '../api';
import { formatLocation } from '../format';
import { describeError, t } from '../i18n';
import { type ProbeState, useStore } from '../store';
import { Sheet } from './Sheet';
import { Button, Field, Segmented } from './ui';

const MODES: { value: UpstreamMode; label: string }[] = [
  { value: 'system', label: t.followSystem },
  { value: 'manual', label: t.manual },
];

const KINDS: { value: ProxyKind; label: string }[] = [
  { value: 'socks5', label: 'SOCKS5' },
  { value: 'http', label: 'HTTP' },
];

export function parsePort(value: string): number {
  return Math.min(Number(value.replace(/\D/g, '').slice(0, 5) || 0), 65535);
}

type Detection = { phase: 'running' } | { phase: 'done'; hop: Hop } | { phase: 'failed'; error: CommandError };

function DetectedProxy({ detection }: { detection: Detection | null }) {
  if (!detection || detection.phase === 'running') {
    return (
      <div className="h-[52px] rounded-lg border border-line bg-surface-2 px-3 py-2 text-[11.5px] text-muted">
        {t.detectedProxy}
      </div>
    );
  }
  if (detection.phase === 'failed') {
    return (
      <div className="flex items-start gap-2 rounded-lg bg-danger-soft px-3 py-2 text-[11.5px] leading-snug text-danger">
        <AlertTriangle className="mt-px size-3.5 shrink-0" />
        <span>{describeError(detection.error)}</span>
      </div>
    );
  }
  const { hop } = detection;
  return (
    <div className="rounded-lg border border-line bg-surface-2 px-3 py-2">
      <div className="text-[11px] leading-4 text-muted">{t.detectedProxy}</div>
      <div className="mt-1 flex items-center gap-2 font-mono text-[12.5px] leading-5">
        <span className="rounded bg-accent-soft px-1.5 text-[10.5px] font-semibold text-accent">
          {hop.kind === 'socks5' ? 'SOCKS5' : 'HTTP'}
        </span>
        {hop.host.includes(':') ? `[${hop.host}]` : hop.host}:{hop.port}
      </div>
    </div>
  );
}

function ProbeResult({ probe }: { probe: ProbeState | null }) {
  if (!probe || probe.phase === 'running') {
    return null;
  }
  if (probe.phase === 'failed') {
    return (
      <div className="flex items-start gap-2 rounded-lg bg-danger-soft px-3 py-2 text-[11.5px] leading-snug text-danger">
        <AlertTriangle className="mt-px size-3.5 shrink-0" />
        <span>{describeError(probe.error)}</span>
      </div>
    );
  }
  const { info } = probe;
  return (
    <div className="flex items-center gap-2.5 rounded-lg bg-ok-soft px-3 py-2 text-[11.5px]">
      <Globe2 className="size-4 shrink-0 text-ok" />
      <div className="min-w-0 flex-1">
        <div className="font-mono text-[12.5px] font-semibold text-text">{info.ip}</div>
        <div className="truncate text-muted" title={info.org}>
          {formatLocation([info.country, info.region, info.city, info.org])}
        </div>
      </div>
      <span className="shrink-0 font-mono text-muted">{info.latencyMs} ms</span>
    </div>
  );
}

export function HopSheet({ which }: { which: 'upstream' | 'landing' }) {
  const config = useStore(s => s.status!.config);
  const saveConfig = useStore(s => s.saveConfig);
  const openSheet = useStore(s => s.openSheet);
  const [hop, setHop] = useState<Hop>(config[which]);
  const [mode, setMode] = useState<UpstreamMode>(config.upstreamMode);
  const [detection, setDetection] = useState<Detection | null>(null);
  const followSystem = which === 'upstream' && mode === 'system';
  const [reveal, setReveal] = useState(false);
  const [probe, setProbe] = useState<ProbeState | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<CommandError | null>(null);
  const update = (patch: Partial<Hop>) => setHop(current => ({ ...current, ...patch }));
  const close = () => openSheet(null);

  useEffect(() => {
    if (!followSystem) {
      return;
    }
    setDetection({ phase: 'running' });
    api.detectSystemProxy().then(
      detected => setDetection({ phase: 'done', hop: detected }),
      raw => setDetection({ phase: 'failed', error: toCommandError(raw) }),
    );
  }, [followSystem]);

  const runProbe = async () => {
    setProbe({ phase: 'running' });
    try {
      const hops =
        which === 'upstream'
          ? [followSystem ? await api.detectSystemProxy() : hop]
          : [await resolveUpstream(config), hop];
      setProbe({ phase: 'done', info: await api.probe(hops) });
    } catch (raw) {
      setProbe({ phase: 'failed', error: toCommandError(raw) });
    }
  };

  const save = async () => {
    setSaving(true);
    const result = await saveConfig(
      which === 'upstream'
        ? { ...config, upstreamMode: mode, upstream: hop }
        : { ...config, landing: hop },
    );
    setSaving(false);
    if (result) {
      setError(result);
    } else {
      close();
    }
  };

  return (
    <Sheet
      title={which === 'upstream' ? t.upstreamTitle : t.landingTitle}
      hint={
        which === 'landing' ? t.landingHint : followSystem ? t.upstreamSystemHint : t.upstreamHint
      }
      error={error}
      saving={saving}
      onSave={save}
      onClose={close}
    >
      <div className="flex items-center justify-between">
        {which === 'upstream' ? (
          <Segmented value={mode} options={MODES} onChange={setMode} />
        ) : (
          <Segmented value={hop.kind} options={KINDS} onChange={kind => update({ kind })} />
        )}
        <Button
          size="sm"
          loading={probe?.phase === 'running'}
          disabled={probe?.phase === 'running'}
          title={which === 'upstream' ? t.probeUpstreamHint : t.probeChainHint}
          onClick={runProbe}
        >
          {probe?.phase === 'running' ? t.probing : t.probe}
        </Button>
      </div>
      {followSystem ? (
        <DetectedProxy detection={detection} />
      ) : (
        <>
          {which === 'upstream' && (
            <Segmented value={hop.kind} options={KINDS} onChange={kind => update({ kind })} />
          )}
          <div className="flex gap-2">
            <Field
              label={t.host}
              className="flex-1"
              value={hop.host}
              autoFocus={!hop.host}
              placeholder={which === 'upstream' ? '127.0.0.1' : 'proxy.example.com'}
              onChange={event => update({ host: event.target.value.trim() })}
            />
            <Field
              label={t.port}
              className="w-24"
              inputMode="numeric"
              value={hop.port || ''}
              placeholder={which === 'upstream' ? '7897' : '1080'}
              onChange={event => update({ port: parsePort(event.target.value) })}
            />
          </div>
          <div className="flex gap-2">
            <Field
              label={t.username}
              className="flex-1"
              value={hop.username}
              placeholder={t.optional}
              onChange={event => update({ username: event.target.value })}
            />
            <Field
              label={t.password}
              className="flex-1"
              type={reveal ? 'text' : 'password'}
              value={hop.password}
              placeholder={t.optional}
              onChange={event => update({ password: event.target.value })}
              trailing={
                <button
                  type="button"
                  tabIndex={-1}
                  onClick={() => setReveal(value => !value)}
                  className="px-2 text-faint hover:text-text"
                >
                  {reveal ? <EyeOff className="size-3.5" /> : <Eye className="size-3.5" />}
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
