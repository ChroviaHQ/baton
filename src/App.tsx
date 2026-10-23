import { useEffect } from 'react';

import { ErrorToast } from './components/ErrorToast';
import { HopSheet } from './components/HopSheet';
import { LocalSheet } from './components/LocalSheet';
import { RelayCard } from './components/RelayCard';
import { SettingsSheet } from './components/SettingsSheet';
import { SystemProxyCard } from './components/SystemProxyCard';
import { TitleBar } from './components/TitleBar';
import { TrafficCard } from './components/TrafficCard';
import { type SheetId, useStore } from './store';

function ActiveSheet({sheet}: {sheet: SheetId;}) {
  switch (sheet) {
    case 'upstream':
    case 'landing':
      return <HopSheet key={sheet} role={sheet} />;
    case 'local':
      return <LocalSheet />;
    case 'settings':
      return <SettingsSheet />;
  }
}

export function App() {
  const init = useStore(s => s.init);
  const ready = useStore(s => !!s.status);
  const sheet = useStore(s => s.sheet);

  useEffect(() => {
    const cleanup = init();
    return () => void cleanup.then(dispose => dispose());
  }, [init]);

  return (
    <div className='flex h-full flex-col'>
      <TitleBar />
      {ready && (
        <main className='flex flex-1 flex-col gap-3 overflow-y-auto px-4.5 pb-4.5'>
          <RelayCard />
          <TrafficCard />
          <SystemProxyCard />
        </main>
      )}
      <ErrorToast />
      {ready && sheet && <ActiveSheet sheet={sheet} />}
    </div>
  );
}
