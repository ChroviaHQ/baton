import { AlertTriangle, Check, Copy, Loader2, X } from 'lucide-react';
import {
  type ButtonHTMLAttributes,
  type InputHTMLAttributes,
  type ReactNode,
  useState,
} from 'react';

import type { CommandError } from '../api';
import { describeError, t } from '../i18n';

export function Card({className = '', children}: {className?: string; children: ReactNode;}) {
  return (
    <section
      className={`flex flex-col rounded-2xl border border-line bg-surface p-4 shadow-card ${className}`}
    >
      {children}
    </section>
  );
}

export function Segmented<T extends string>(
  {value, options, onChange}: {
    value: T;
    options: {value: T; label: string;}[];
    onChange: (value: T) => void;
  },
) {
  return (
    <div className='inline-flex rounded-lg bg-surface-2 p-0.5'>
      {options.map(option => (
        <button
          key={option.value}
          type='button'
          aria-pressed={option.value === value}
          onClick={() => onChange(option.value)}
          className={`rounded-md px-2.5 py-1 text-[11.5px] font-medium transition ${
            option.value === value ? 'bg-surface text-text shadow-sm' : 'text-muted hover:text-text'
          }`}
        >
          {option.label}
        </button>
      ))}
    </div>
  );
}

export function TextField(
  {label, hint, className = '', trailing, ...props}: InputHTMLAttributes<HTMLInputElement> & {
    label: string;
    hint?: string;
    trailing?: ReactNode;
  },
) {
  return (
    <label className={`flex flex-col gap-1 ${className}`}>
      <span className='text-[11px] font-medium text-muted'>{label}</span>
      <span className='flex items-center rounded-lg border border-line bg-surface-2 transition focus-within:border-accent focus-within:ring-3 focus-within:ring-accent-soft'>
        <input
          {...props}
          spellCheck={false}
          autoCorrect='off'
          autoCapitalize='off'
          className='h-8 w-full min-w-0 bg-transparent px-2.5 font-mono text-[12.5px] text-text outline-none placeholder:text-faint'
        />
        {trailing}
      </span>
      {hint && <span className='text-[11px] leading-4 text-muted'>{hint}</span>}
    </label>
  );
}

function Switch(
  {label, checked, onChange, disabled}: {
    label: string;
    checked: boolean;
    onChange: (checked: boolean) => void;
    disabled?: boolean;
  },
) {
  return (
    <button
      type='button'
      role='switch'
      aria-label={label}
      aria-checked={checked}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={`flex h-5.5 w-9.5 shrink-0 items-center rounded-full p-0.75 transition disabled:opacity-50 ${
        checked ? 'bg-accent' : 'bg-faint/40'
      }`}
    >
      <span
        className={`size-4 rounded-full bg-white shadow transition-transform ${
          checked ? 'translate-x-full' : ''
        }`}
      />
    </button>
  );
}

const BUTTON_VARIANTS = {
  primary: 'border-transparent bg-accent font-semibold text-on-accent hover:brightness-110',
  secondary:
    'border-line bg-surface-2 font-medium text-text hover:border-accent/50 hover:text-accent',
};

const BUTTON_SIZES = {sm: 'h-7 px-2.5 text-[11.5px]', md: 'h-8 px-3.5 text-[12.5px]'};

export function Button(
  {children, loading, variant = 'secondary', size = 'md', className = '', ...props}:
    & ButtonHTMLAttributes<HTMLButtonElement>
    & {
      loading?: boolean;
      variant?: keyof typeof BUTTON_VARIANTS;
      size?: keyof typeof BUTTON_SIZES;
    },
) {
  return (
    <button
      type='button'
      {...props}
      className={`inline-flex shrink-0 items-center justify-center gap-1.5 rounded-lg border transition disabled:pointer-events-none disabled:opacity-60 ${
        BUTTON_VARIANTS[variant]
      } ${BUTTON_SIZES[size]} ${className}`}
    >
      {loading && <Loader2 className='size-3.5 animate-spin' />}
      {children}
    </button>
  );
}

export function IconButton(
  {children, className = '', ...props}: ButtonHTMLAttributes<HTMLButtonElement>,
) {
  return (
    <button
      type='button'
      {...props}
      className={`grid size-6.5 shrink-0 place-items-center rounded-md text-muted transition hover:bg-surface-2 hover:text-text ${className}`}
    >
      {children}
    </button>
  );
}

export function LineAligned({children}: {children: ReactNode;}) {
  return <span className='flex h-lh shrink-0 items-center'>{children}</span>;
}

export function CloseButton({onClick}: {onClick: () => void;}) {
  return (
    <IconButton title={t.close} onClick={onClick}>
      <X className='size-4' />
    </IconButton>
  );
}

export function ToggleRow(
  {title, hint, checked, disabled, onChange}: {
    title: string;
    hint: string;
    checked: boolean;
    disabled?: boolean;
    onChange: (checked: boolean) => void;
  },
) {
  return (
    <div className='flex items-center justify-between gap-4'>
      <div className='flex min-w-0 flex-col gap-0.5'>
        <div className='text-[12.5px] leading-5 font-medium'>{title}</div>
        <div className='text-[11px] leading-4 text-muted'>{hint}</div>
      </div>
      <Switch label={title} checked={checked} disabled={disabled} onChange={onChange} />
    </div>
  );
}

export function ErrorNote({error}: {error: CommandError;}) {
  return (
    <div className='flex items-start gap-2 rounded-lg bg-danger-soft px-3 py-2 text-[11.5px] leading-snug text-danger'>
      <LineAligned>
        <AlertTriangle className='size-3.5' />
      </LineAligned>
      <span className='wrap-anywhere'>{describeError(error)}</span>
    </div>
  );
}

export function CopyButton({value, children}: {value: string; children: ReactNode;}) {
  const [copied, setCopied] = useState(false);
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(value);
    } catch {
      return;
    }
    setCopied(true);
    setTimeout(() => setCopied(false), 1200);
  };
  return (
    <button
      type='button'
      onClick={copy}
      title={copied ? t.copied : t.copy}
      className='group flex w-full items-center gap-2 rounded-lg border border-line bg-surface-2 px-3 py-2.5 text-left transition hover:border-accent/40'
    >
      {children}
      {copied
        ? <Check className='size-3.5 shrink-0 text-ok' />
        : <Copy className='size-3.5 shrink-0 text-faint group-hover:text-accent' />}
    </button>
  );
}
