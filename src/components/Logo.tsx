const TRACK = 'M8 6.5h8a5.5 5.5 0 0 1 0 11H8a5.5 5.5 0 0 1 0-11Z';

export function Logo({ size = 20, animated = false }: { size?: number; animated?: boolean }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" aria-hidden="true">
      <g
        transform="rotate(-28 12 12)"
        fill="none"
        stroke="var(--accent)"
        strokeWidth="2.2"
        strokeLinecap="round"
      >
        <path d={TRACK} opacity={animated ? 0.3 : 1} />
        {animated && (
          <path d={TRACK} pathLength={100} strokeDasharray="24 76" className="logo-run" />
        )}
        <path d="M10 12h4" />
      </g>
    </svg>
  );
}
