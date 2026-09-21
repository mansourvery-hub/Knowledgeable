import { confidenceStatus } from '../../graphUtils';

export interface ConfidenceRingProps {
  value: number | null | undefined;
  size?: number;
  label?: boolean;
}

const CLASS = {
  healthy: 'k-ring--solid',
  review: 'k-ring--building',
  unseen: 'k-ring--unseen',
} as const;

export default function ConfidenceRing({ value, size = 22 }: ConfidenceRingProps) {
  const status = confidenceStatus(value ?? null);
  const v = status === 'unseen' ? 0 : Math.min(1, Math.max(0, value ?? 0));
  const sw = Math.max(2.2, size * 0.13);
  const r = (size - sw) / 2;
  const c = size / 2;
  const len = 2 * Math.PI * r;
  return (
    <svg
      className={`k-ring ${CLASS[status]}`}
      width={size}
      height={size}
      viewBox={`0 0 ${size} ${size}`}
      aria-hidden="true"
    >
      <circle className="k-ring__track" cx={c} cy={c} r={r} strokeWidth={sw} />
      {v > 0 && (
        <circle
          className="k-ring__arc"
          cx={c}
          cy={c}
          r={r}
          strokeWidth={sw}
          strokeDasharray={`${(len * v).toFixed(2)} ${len.toFixed(2)}`}
          transform={`rotate(-90 ${c} ${c})`}
        />
      )}
    </svg>
  );
}
