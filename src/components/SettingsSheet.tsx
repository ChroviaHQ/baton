import { useEffect, useState } from 'react';

import { api, toCommandError } from '../api';
import { formatEndpoint, parsePort } from '../format';
import { t } from '../i18n';
import { isMac } from '../platform';
import { useStore } from '../store';
import { Sheet } from './Sheet';
import { CopyButton, TextField, ToggleRow } from './ui';

function LanAddress({ip, port}: {ip: string | null; port: number;}) {
  if (!ip) {
    return <div className='text-[11.5px] text-muted'>{t.noLanIp}</div>;
  }
  const address = formatEndpoint(ip, port);
  return (
    <CopyButton value={address}>
      <span className='flex-1 text-[11.5px] text-muted'>{t.lanAddress}</span>
      <span className='font-mono text-[12px]'>{address}</span>
    </CopyButton>
  );
}

export function SettingsSheet() {
  const config = useStore(s => s.status!.config);
  const lanIp = useStore(s => s.status!.lanIp);
  const saveConfig = useStore(s => s.saveConfig);
  const setSheetError = useStore(s => s.setSheetError);
  const [listenPort, setListenPort] = useState(config.listenPort);
  const [allowLan, setAllowLan] = useState(config.allowLan);
  const [showInDock, setShowInDock] = useState(config.showInDock);
  const [startOnLaunch, setStartOnLaunch] = useState(config.startOnLaunch);
  const [launchAtLogin, setLaunchAtLogin] = useState<boolean | null>(null);
  const [initialLaunchAtLogin, setInitialLaunchAtLogin] = useState<boolean | null>(null);

  useEffect(() => {
    api.getLaunchAtLogin().then(enabled => {
      setLaunchAtLogin(enabled);
      setInitialLaunchAtLogin(enabled);
    }, raw => setSheetError(toCommandError(raw)));
  }, [setSheetError]);

  const save = async () => {
    const error = await saveConfig({listenPort, allowLan, showInDock, startOnLaunch});
    if (error || launchAtLogin === null || launchAtLogin === initialLaunchAtLogin) {
      return error;
    }
    try {
      await api.setLaunchAtLogin(launchAtLogin);
      return null;
    } catch (raw) {
      return toCommandError(raw);
    }
  };

  return (
    <Sheet title={t.settings} onSave={save}>
      <TextField
        label={t.listenPort}
        className='w-28'
        inputMode='numeric'
        value={listenPort || ''}
        placeholder='1080'
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
        disabled={launchAtLogin === null}
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
        title={t.startOnLaunch}
        hint={t.startOnLaunchHint}
        checked={startOnLaunch}
        onChange={setStartOnLaunch}
      />
    </Sheet>
  );
}
