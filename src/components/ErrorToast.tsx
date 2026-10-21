import { AlertTriangle, X } from 'lucide-react';

import { describeError, t } from '../i18n';
import { useStore } from '../store';
import { IconButton } from './ui';

export function ErrorToast() {
  const error = useStore(s => s.error);
  const dismiss = useStore(s => s.dismissError);
  if (!error) {
    return null;
  }
  return (
    <div
      role='alert'
      className='toast-in fixed inset-x-4.5 bottom-4.5 z-20 flex items-start gap-2.5 rounded-xl border border-danger/25 bg-surface px-3.5 py-2.5 text-[12px] shadow-card'
    >
      <AlertTriangle className='mt-px size-4 shrink-0 text-danger' />
      <span className='flex-1 leading-snug'>{describeError(error)}</span>
      <IconButton title={t.close} onClick={dismiss} className='-mt-1 -mr-1.5'>
        <X className='size-4' />
      </IconButton>
    </div>
  );
}
