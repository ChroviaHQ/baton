import { type ReactNode, useEffect, useState } from 'react';

import type { CommandError } from '../api';
import { t } from '../i18n';
import { useStore } from '../store';
import { Button, CloseButton, ErrorNote } from './ui';

export function Sheet(
  {title, hint, children, onSave}: {
    title: string;
    hint?: string;
    children: ReactNode;
    onSave: () => Promise<CommandError | null>;
  },
) {
  const closeSheet = useStore(s => s.closeSheet);
  const error = useStore(s => s.sheetError);
  const setError = useStore(s => s.setSheetError);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        closeSheet();
      }
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [closeSheet]);

  const submit = async () => {
    setSaving(true);
    setError(null);
    const result = await onSave();
    setSaving(false);
    if (result) {
      setError(result);
    } else {
      closeSheet();
    }
  };

  return (
    <div className='fixed inset-0 z-10 flex flex-col justify-end'>
      <div className='sheet-fade absolute inset-0 bg-black/35' onClick={closeSheet} />
      <form
        className='sheet-up relative flex max-h-[88%] flex-col rounded-t-2xl border-t border-line bg-surface shadow-card'
        onSubmit={event => {
          event.preventDefault();
          void submit();
        }}
      >
        <header className='flex flex-col gap-0.5 px-4 pt-4'>
          <div className='flex items-center justify-between gap-3'>
            <h2 className='text-[14px] font-semibold tracking-tight'>{title}</h2>
            <CloseButton onClick={closeSheet} />
          </div>
          {hint && <p className='text-[11.5px] leading-snug text-muted'>{hint}</p>}
        </header>
        <div className='flex min-h-0 flex-col gap-3 overflow-y-auto px-4 pt-3.5 pb-1'>
          {children}
        </div>
        <footer className='flex flex-col gap-3 px-4 pt-3 pb-4'>
          {error && <ErrorNote error={error} />}
          <div className='flex justify-end gap-2'>
            <Button onClick={closeSheet}>{t.cancel}</Button>
            <Button type='submit' variant='primary' loading={saving} disabled={saving}>
              {t.save}
            </Button>
          </div>
        </footer>
      </form>
    </div>
  );
}
