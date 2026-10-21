import { AlertTriangle, X } from 'lucide-react';
import { useEffect, type ReactNode } from 'react';

import type { CommandError } from '../api';
import { describeError, t } from '../i18n';
import { Button, IconButton } from './ui';

export function Sheet({
  title,
  hint,
  children,
  error,
  saving,
  onSave,
  onClose,
}: {
  title: string;
  hint?: string;
  children: ReactNode;
  error?: CommandError | null;
  saving?: boolean;
  onSave?: () => void;
  onClose: () => void;
}) {
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') {
        onClose();
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [onClose]);

  return (
    <div className="fixed inset-0 z-10 flex flex-col justify-end">
      <div className="sheet-fade absolute inset-0 bg-black/35" onClick={onClose} />
      <form
        className="sheet-up relative max-h-[88%] overflow-y-auto rounded-t-2xl border-t border-line bg-surface shadow-card"
        onSubmit={event => {
          event.preventDefault();
          onSave?.();
        }}
      >
        <header className="flex items-start justify-between gap-3 px-4 pt-4">
          <div>
            <h2 className="text-[14px] font-semibold tracking-tight">{title}</h2>
            {hint && <p className="mt-0.5 text-[11.5px] leading-snug text-muted">{hint}</p>}
          </div>
          <IconButton title={t.close} onClick={onClose} className="-mt-1 -mr-1.5">
            <X className="size-4" />
          </IconButton>
        </header>
        <div className="space-y-3 px-4 pt-3.5">{children}</div>
        {error && (
          <div className="mx-4 mt-3 flex items-start gap-2 rounded-lg bg-danger-soft px-3 py-2 text-[11.5px] leading-snug text-danger">
            <AlertTriangle className="mt-px size-3.5 shrink-0" />
            <span>{describeError(error)}</span>
          </div>
        )}
        <footer className="flex justify-end gap-2 p-4">
          <Button onClick={onClose}>{onSave ? t.cancel : t.close}</Button>
          {onSave && (
            <Button type="submit" variant="primary" loading={saving} disabled={saving}>
              {t.save}
            </Button>
          )}
        </footer>
      </form>
    </div>
  );
}
