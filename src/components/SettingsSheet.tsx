import { Check, Copy } from 'lucide-react';
import { useEffect, useState } from 'react';

import { api, toCommandError, type CommandError } from '../api';
import { t } from '../i18n';
import { useStore } from '../store';
import { parsePort } from './HopSheet';
import { Sheet } from './Sheet';
import { Field, Switch } from './ui';

const isMac = navigator.userAgent.includes('Mac');

function ToggleRow({
  title,
  hint,
  checked,
  onChange,
}: {
  title: string;
  hint: string;
  checked: boolean;
  onChange: (value: boolean) => void;
}) {
  return (
    <div className="flex items-center justify-between gap-4">
      <div>
        <div className="text-[12.5px] leading-5 font-medium">{title}</div>
        <div className="mt-0.5 text-[11px] leading-4 text-muted">{hint}</div>
      </div>
      <Switch checked={checked} onChange={onChange} />
    </div>
  );
}

function LanAddress({ ip, port }: { ip: string | null; port: number }) {
  const [copied, setCopied] = useState(false);
  if (!ip) {
    return <div className="text-[11.5px] text-muted">{t.noLanIp}</div>;
  }
  const value = `${ip}:${port}`;
  const copy = async () => {
    await navigator.clipboard.writeText(value);
    setCopied(true);
    setTimeout(() => setCopied(false), 1200);
  };
  return (
    <button
      type="button"
      onClick={copy}
      title={copied ? t.copied : t.copy}
      className="group flex w-full items-center justify-between gap-2 rounded-lg border border-line bg-surface-2 px-3 py-2 text-left transition hover:border-accent/40"
    >
      <span className="text-[11.5px] text-muted">{t.lanAddress}</span>
      <span className="flex items-center gap-2 font-mono text-[12px]">
        {value}
        {copied ? (
          <Check className="size-3.5 text-ok" />
        ) : (
          <Copy className="size-3.5 text-faint group-hover:text-accent" />
        )}
      </span>
    </button>
  );
}

export function SettingsSheet() {
  const config = useStore(s => s.status!.config);
  const lanIp = useStore(s => s.status!.lanIp);
  const saveConfig = useStore(s => s.saveConfig);
  const openSheet = useStore(s => s.openSheet);
  const [listenPort, setListenPort] = useState(config.listenPort);
  const [allowLan, setAllowLan] = useState(config.allowLan);
  const [autoConnect, setAutoConnect] = useState(config.autoConnect);
  const [showInDock, setShowInDock] = useState(config.showInDock);
  const [launchAtLogin, setLaunchAtLogin] = useState<boolean | null>(null);
  const [savedLaunchAtLogin, setSavedLaunchAtLogin] = useState<boolean | null>(null);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<CommandError | null>(null);
  const close = () => openSheet(null);

  useEffect(() => {
    api.getLaunchAtLogin().then(
      enabled => {
        setLaunchAtLogin(enabled);
        setSavedLaunchAtLogin(enabled);
      },
      raw => setError(toCommandError(raw)),
    );
  }, []);

  const save = async () => {
    setSaving(true);
    let result = await saveConfig({ ...config, listenPort, allowLan, autoConnect, showInDock });
    if (!result && launchAtLogin !== null && launchAtLogin !== savedLaunchAtLogin) {
      try {
        setSavedLaunchAtLogin(await api.setLaunchAtLogin(launchAtLogin));
      } catch (raw) {
        result = toCommandError(raw);
      }
    }
    setSaving(false);
    if (result) {
      setError(result);
    } else {
      close();
    }
  };

  return (
    <Sheet title={t.settings} error={error} saving={saving} onSave={save} onClose={close}>
      <Field
        label={t.listenPort}
        className="w-28"
        inputMode="numeric"
        value={listenPort || ''}
        placeholder="6868"
        onChange={event => setListenPort(parsePort(event.target.value))}
      />
      <ToggleRow
        title={t.allowLan}
        hint={t.allowLanHint}
        checked={allowLan}
        onChange={setAllowLan}
      />
      {allowLan && <LanAddress ip={lanIp} port={listenPort || config.listenPort} />}
      <ToggleRow
        title={t.launchAtLogin}
        hint={t.launchAtLoginHint}
        checked={launchAtLogin ?? false}
        onChange={setLaunchAtLogin}
      />
      {isMac && (
        <ToggleRow
          title={t.showInDock}
          hint={t.showInDockHint}
          checked={showInDock}
          onChange={setShowInDock}
        />
      )}
      <ToggleRow
        title={t.autoConnect}
        hint={t.autoConnectHint}
        checked={autoConnect}
        onChange={setAutoConnect}
      />
    </Sheet>
  );
}
