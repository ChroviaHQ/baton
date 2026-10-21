import { t } from '../i18n';
import { useStore } from '../store';
import { Card, ToggleRow } from './ui';

export function SystemProxyCard() {
  const checked = useStore(s => s.status!.config.useAsSystemProxy);
  const busy = useStore(s => s.busy);
  const setUseAsSystemProxy = useStore(s => s.setUseAsSystemProxy);
  return (
    <Card>
      <ToggleRow
        title={t.useAsSystemProxy}
        hint={t.useAsSystemProxyHint}
        checked={checked}
        disabled={busy}
        onChange={setUseAsSystemProxy}
      />
    </Card>
  );
}
