import { useEffect, useRef, useState } from 'react';

import { api, type LanAddress, toCommandError } from '../api';
import { formatEndpoint } from '../format';
import { t } from '../i18n';
import { isMac } from '../platform';
import { useStore } from '../store';
import { Sheet } from './Sheet';
import { CopyButton, TextField, ToggleRow } from './ui';

function LanAddresses({addresses, port}: {addresses: LanAddress[]; port: number;}) {
  if (addresses.length === 0) {
    return <div className='text-[11.5px] text-muted'>{t.noLanAddress}</div>;
  }
  return (
    <div className='flex flex-col gap-1.5'>
      {addresses.map(address => {
        const endpoint = formatEndpoint(address.ip, port);
        return (
          <CopyButton key={`${address.interface} ${address.ip}`} value={endpoint}>
            <span className='flex-1 truncate text-[11.5px] text-muted'>{address.interface}</span>
            <span className='font-mono text-[12px]'>{endpoint}</span>
          </CopyButton>
        );
      })}
    </div>
  );
}

export function SettingsSheet() {
  const config = useStore(s => s.status!.config);
  const saveConfig = useStore(s => s.saveConfig);
  const setSheetError = useStore(s => s.setSheetError);
  const [allowLan, setAllowLan] = useState(config.allowLan);
  const [showInDock, setShowInDock] = useState(config.showInDock);
  const [startOnLaunch, setStartOnLaunch] = useState(config.startOnLaunch);
  const [bypassChinaDomains, setBypassChinaDomains] = useState(config.bypassChinaDomains);
  const [bypassText, setBypassText] = useState(config.bypass.join(', '));
  const [lanAddresses, setLanAddresses] = useState<LanAddress[] | null>(null);
  const [launchAtLogin, setLaunchAtLogin] = useState<boolean | null>(null);
  const initialLaunchAtLogin = useRef<boolean | null>(null);

  useEffect(() => {
    api.getLanAddresses().then(setLanAddresses);
  }, []);

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
      bypassChinaDomains,
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
      {allowLan && lanAddresses && (
        <LanAddresses addresses={lanAddresses} port={config.listenPort} />
      )}
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
      <ToggleRow
        title={t.bypassChinaDomains}
        hint={t.bypassChinaDomainsHint}
        checked={bypassChinaDomains}
        onChange={setBypassChinaDomains}
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
