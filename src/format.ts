const UNITS = ['B', 'KB', 'MB', 'GB', 'TB'];

function formatBytes(value: number): string {
  let size = value;
  let unit = 0;
  while (size >= 1024 && unit < UNITS.length - 1) {
    size /= 1024;
    unit += 1;
  }
  const digits = unit === 0 || size >= 100 ? 0 : 1;
  return `${size.toFixed(digits)} ${UNITS[unit]}`;
}

export function formatRate(value: number): string {
  return `${formatBytes(value)}/s`;
}

export function formatEndpoint(host: string, port: number): string {
  return host.includes(':') ? `[${host}]:${port}` : `${host}:${port}`;
}

export function formatLocation(parts: string[]): string {
  return parts.filter(Boolean).join(' · ');
}

export function parsePort(value: string): number {
  return Math.min(Number(value.replace(/\D/g, '').slice(0, 5) || 0), 65535);
}
