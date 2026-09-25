import type { KeyboardEvent } from 'react';

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
  const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === 'ArrowRight' || event.key === 'ArrowDown') {
      event.preventDefault();
      onSelect('review');
    } else if (event.key === 'ArrowLeft' || event.key === 'ArrowUp') {
      event.preventDefault();
      onSelect('all');
    }
  };

  return (
    <div
      className="k-seg"
      role="group"
      aria-label={label}
      onKeyDown={handleKeyDown}
    >
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
