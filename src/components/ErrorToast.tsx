import { AlertTriangle } from 'lucide-react';

import { describeError } from '../i18n';
import { useStore } from '../store';
import { CloseButton, LineAligned } from './ui';

export function ErrorToast() {
  const error = useStore(s => s.error);
  const dismiss = useStore(s => s.dismissError);
  if (!error) {
    return null;
  }
  return (
    <div
      role='alert'
      className='toast-in fixed inset-x-4.5 bottom-4.5 z-20 flex items-start gap-2.5 rounded-xl border border-danger/25 bg-surface px-3.5 py-2.5 text-[12px] leading-5 shadow-card'
    >
      <LineAligned>
        <AlertTriangle className='size-4 text-danger' />
      </LineAligned>
      <span className='flex-1 wrap-anywhere'>{describeError(error)}</span>
      <LineAligned>
        <CloseButton onClick={dismiss} />
      </LineAligned>
    </div>
  );
}
