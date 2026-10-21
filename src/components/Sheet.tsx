import { X } from 'lucide-react';
import { type ReactNode, useEffect, useState } from 'react';

import type { CommandError } from '../api';
import { t } from '../i18n';
import { useStore } from '../store';
import { Button, ErrorNote, IconButton } from './ui';

export function Sheet(
  {title, hint, children, onSave}: {
    title: string;
    hint?: string;
    children: ReactNode;
    onSave?: () => Promise<CommandError | null>;
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
    if (!onSave) {
      return;
    }
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
        className='sheet-up relative max-h-[88%] overflow-y-auto rounded-t-2xl border-t border-line bg-surface shadow-card'
        onSubmit={event => {
          event.preventDefault();
          void submit();
        }}
      >
        <header className='flex items-start justify-between gap-3 px-4 pt-4'>
          <div>
            <h2 className='text-[14px] font-semibold tracking-tight'>{title}</h2>
            {hint && <p className='mt-0.5 text-[11.5px] leading-snug text-muted'>{hint}</p>}
          </div>
          <IconButton title={t.close} onClick={closeSheet} className='-mt-1 -mr-1.5'>
            <X className='size-4' />
          </IconButton>
        </header>
        <div className='space-y-3 px-4 pt-3.5'>{children}</div>
        {error && (
          <div className='px-4 pt-3'>
            <ErrorNote error={error} />
          </div>
        )}
        <footer className='flex justify-end gap-2 p-4'>
          <Button onClick={closeSheet}>{onSave ? t.cancel : t.close}</Button>
          {onSave && (
            <Button type='submit' variant='primary' loading={saving} disabled={saving}>
              {t.save}
            </Button>
          )}
        </footer>
      </form>
    </div>
  );
}
