import { useEffect, useRef, useState } from 'react';

import { api, toCommandError } from '../api';
import { formatEndpoint } from '../format';
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
  const [allowLan, setAllowLan] = useState(config.allowLan);
  const [showInDock, setShowInDock] = useState(config.showInDock);
  const [startOnLaunch, setStartOnLaunch] = useState(config.startOnLaunch);
  const [bypassText, setBypassText] = useState(config.bypass.join(', '));
  const [launchAtLogin, setLaunchAtLogin] = useState<boolean | null>(null);
  const initialLaunchAtLogin = useRef<boolean | null>(null);

  useEffect(() => {
    api.getLaunchAtLogin().then(enabled => {
      setLaunchAtLogin(enabled);
      initialLaunchAtLogin.current = enabled;
    }, raw => setSheetError(toCommandError(raw)));
  }, [setSheetError]);

  const save = async () => {
    const error = await saveConfig({
      allowLan,
      showInDock,
      startOnLaunch,
      bypass: bypassText.split(/[\s,]+/).filter(Boolean),
    });
    if (error || launchAtLogin === null || launchAtLogin === initialLaunchAtLogin.current) {
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
      <ToggleRow
        title={t.allowLan}
        hint={t.allowLanHint}
        checked={allowLan}
        onChange={setAllowLan}
      />
      {allowLan && <LanAddress ip={lanIp} port={config.listenPort} />}
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
      <TextField
        label={t.bypass}
        hint={t.bypassHint}
        value={bypassText}
        placeholder='*.corp.example, 100.64.0.0/10'
        onChange={event => setBypassText(event.target.value)}
      />
    </Sheet>
  );
}
