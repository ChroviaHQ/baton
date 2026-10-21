import { ArrowDown, ArrowUp, Cable } from 'lucide-react';

import { formatRate } from '../format';
import { t } from '../i18n';
import { useStore } from '../store';
import { Card } from './ui';

const WIDTH = 400;
const HEIGHT = 100;

function path(values: number[], max: number): string {
  const step = WIDTH / (values.length - 1);
  return values
    .map((value, index) => {
      const x = (index * step).toFixed(1);
      const y = (HEIGHT - 2 - (value / max) * (HEIGHT - 6)).toFixed(1);
      return `${index === 0 ? 'M' : 'L'}${x} ${y}`;
    })
    .join(' ');
}

function Sparkline() {
  const history = useStore(s => s.history);
  const max = Math.max(1024, ...history.map(s => Math.max(s.up, s.down)));
  const down = path(history.map(s => s.down), max);
  const up = path(history.map(s => s.up), max);
  return (
    <svg viewBox={`0 0 ${WIDTH} ${HEIGHT}`} preserveAspectRatio="none" className="h-[100px] w-full">
      <path d={`${down} L${WIDTH} ${HEIGHT} L0 ${HEIGHT} Z`} fill="var(--accent)" fillOpacity="0.12" />
      <path d={down} fill="none" stroke="var(--accent)" strokeWidth="1.5" vectorEffect="non-scaling-stroke" />
      <path
        d={up}
        fill="none"
        stroke="var(--muted)"
        strokeWidth="1.25"
        strokeDasharray="3 3"
        vectorEffect="non-scaling-stroke"
      />
    </svg>
  );
}

export function TrafficCard() {
  const stats = useStore(s => s.stats);
  return (
    <Card className="p-4">
      <div className="flex h-5 items-center gap-4 font-mono text-[13px] font-semibold tabular-nums">
        <span className="flex items-center gap-1" title={t.download}>
          <ArrowDown className="size-3.5 text-accent" />
          {formatRate(stats?.downRate ?? 0)}
        </span>
        <span className="flex items-center gap-1" title={t.upload}>
          <ArrowUp className="size-3.5 text-muted" />
          {formatRate(stats?.upRate ?? 0)}
        </span>
        <span
          className="ml-auto flex items-center gap-1 text-muted"
          title={`${t.activeConnections} / ${t.failedConnections}`}
        >
          <Cable className="size-3.5" />
          <span className="text-text">{stats?.active ?? 0}</span>
          {!!stats?.failed && <span className="text-danger">/ {stats.failed}</span>}
        </span>
      </div>
      <div className="pt-3">
        <Sparkline />
      </div>
    </Card>
  );
}
