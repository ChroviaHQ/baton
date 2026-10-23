import type { CommandError } from './api';
import en from './locales/en.json';
import zh from './locales/zh.json';

type Messages = typeof en;

const isZh = navigator.language.toLowerCase().startsWith('zh');
export const t: Messages = isZh ? zh : en;
document.documentElement.lang = isZh ? 'zh-CN' : 'en';

export function interpolate(template: string, values: Record<string, string | number>): string {
  return template.replace(/\{(\w+)\}/g, (match, key: string) => String(values[key] ?? match));
}

export function describeError(error: CommandError): string {
  const errors: Record<string, string> = t.errors;
  const hop = error.hop === null ? '' : t.hopNames[error.hop] ?? '';
  const detail = (error.reply !== null && t.socks5Replies[error.reply]) || error.detail;
  return interpolate(errors[error.code] ?? errors.unknown, {hop, detail});
}
