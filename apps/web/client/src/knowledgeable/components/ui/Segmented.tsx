export type SegmentedValue = 'all' | 'review';

export interface SegmentedProps {
  allCount: number;
  reviewCount: number;
  selected: SegmentedValue;
  onSelect: (value: SegmentedValue) => void;
  label?: string;
}

export default function Segmented({
  allCount,
  reviewCount,
  selected,
  onSelect,
  label = 'Filter',
}: SegmentedProps) {
  return (
    <div className="k-seg" role="group" aria-label={label}>
      <button
        type="button"
        aria-pressed={selected === 'all'}
        onClick={() => onSelect('all')}
      >
        All {allCount}
      </button>
      <button
        type="button"
        aria-pressed={selected === 'review'}
        onClick={() => onSelect('review')}
      >
        Needs review {reviewCount}
      </button>
    </div>
  );
}
