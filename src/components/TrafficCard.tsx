import { ArrowDown, ArrowUp, Cable } from 'lucide-react';

import { formatRate } from '../format';
import { t } from '../i18n';
import { useStore } from '../store';
import { Card } from './ui';

const CHART_WIDTH = 400;
const CHART_HEIGHT = 100;

function linePath(values: number[], max: number): string {
  const step = CHART_WIDTH / (values.length - 1);
  return values.map((value, index) => {
    const x = (index * step).toFixed(1);
    const y = (CHART_HEIGHT - 2 - (value / max) * (CHART_HEIGHT - 6)).toFixed(1);
    return `${index === 0 ? 'M' : 'L'}${x} ${y}`;
  }).join(' ');
}

function TrafficChart() {
  const rateHistory = useStore(s => s.rateHistory);
  const max = Math.max(1024, ...rateHistory.map(s => Math.max(s.up, s.down)));
  const down = linePath(rateHistory.map(s => s.down), max);
  const up = linePath(rateHistory.map(s => s.up), max);
  return (
    <svg
      viewBox={`0 0 ${CHART_WIDTH} ${CHART_HEIGHT}`}
      preserveAspectRatio='none'
      className='h-25 w-full'
    >
      <path
        d={`${down} L${CHART_WIDTH} ${CHART_HEIGHT} L0 ${CHART_HEIGHT} Z`}
        fill='var(--accent)'
        fillOpacity='0.12'
      />
      <path
        d={down}
        fill='none'
        stroke='var(--accent)'
        strokeWidth='1.5'
        vectorEffect='non-scaling-stroke'
      />
      <path
        d={up}
        fill='none'
        stroke='var(--muted)'
        strokeWidth='1.25'
        strokeDasharray='3 3'
        vectorEffect='non-scaling-stroke'
      />
    </svg>
  );
}

export function TrafficCard() {
  const stats = useStore(s => s.stats);
  return (
    <Card className='gap-3'>
      <div className='flex h-5 items-center gap-4 font-mono text-[13px] font-semibold tabular-nums'>
        <span className='flex items-center gap-1' title={t.download}>
          <ArrowDown className='size-3.5 text-accent' />
          {formatRate(stats?.downRate ?? 0)}
        </span>
        <span className='flex items-center gap-1' title={t.upload}>
          <ArrowUp className='size-3.5 text-muted' />
          {formatRate(stats?.upRate ?? 0)}
        </span>
        <span className='ml-auto flex items-center gap-1 text-muted' title={t.connections}>
          <Cable className='size-3.5' />
          <span className='text-text'>{stats?.active ?? 0}</span>
          {!!stats?.failed && <span className='text-danger'>/ {stats.failed}</span>}
        </span>
      </div>
      <TrafficChart />
    </Card>
  );
}
