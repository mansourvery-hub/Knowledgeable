import ConfidenceRing from './ConfidenceRing';
import { formatConfidence } from '../../graphUtils';

export interface ConceptRowProps {
  name: string;
  value: number | null | undefined;
  selected?: boolean;
  sub?: string;
  onSelect?: () => void;
  testId?: string;
}

export default function ConceptRow({
  name,
  value,
  selected,
  sub,
  onSelect,
  testId,
}: ConceptRowProps) {
  return (
    <li>
      <button
        type="button"
        className="k-row"
        aria-current={selected ? 'true' : undefined}
        onClick={onSelect}
        data-testid={testId}
        title={name}
      >
        <ConfidenceRing value={value} size={22} />
        {sub ? (
          <span className="k-row__body">
            <span className="k-row__name">{name}</span>
            <span className="k-row__sub">{sub}</span>
          </span>
        ) : (
          <span className="k-row__name">{name}</span>
        )}
        <span className="k-row__pct">{formatConfidence(value)}</span>
      </button>
    </li>
  );
}
