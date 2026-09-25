import type { KeyboardEvent } from 'react';

export interface SearchFieldProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  ariaLabel?: string;
  testId?: string;
  onSubmit?: () => void;
  disabled?: boolean;
}

/** `graph-search-input` -> `graph-search`: one prop keeps both testids working. */
export function searchButtonTestId(inputTestId?: string): string | undefined {
  if (!inputTestId) return undefined;
  return inputTestId.endsWith('-input') ? inputTestId.slice(0, -6) : `${inputTestId}-button`;
}

export default function SearchField({
  value, onChange, placeholder, ariaLabel, testId, onSubmit, disabled,
}: SearchFieldProps) {
  const submit = () => { if (!disabled) onSubmit?.(); };
  const handleKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'Enter') {
      event.preventDefault();
      submit();
    } else if (event.key === 'Escape' && value) {
      event.preventDefault();
      onChange('');
    }
  };
  return (
    <label className="k-field">
      <button
        type="button"
        className="k-field__btn"
        aria-label="Search"
        data-testid={searchButtonTestId(testId)}
        onClick={(event) => {
          event.stopPropagation();
          submit();
        }}
        disabled={disabled}
      >
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" aria-hidden="true"><circle cx="11" cy="11" r="7" /><path d="M20 20l-3.5-3.5" /></svg>
      </button>
      <input
        className="k-input"
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={placeholder}
        aria-label={ariaLabel || placeholder || 'Search'}
        data-testid={testId}
        spellCheck={false}
        disabled={disabled}
        onKeyDown={handleKeyDown}
      />
      {value && !disabled && (
        <button
          type="button"
          className="k-field__clear"
          aria-label="Clear search"
          onClick={(event) => {
            event.stopPropagation();
            onChange('');
          }}
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" aria-hidden="true"><line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" /></svg>
        </button>
      )}
    </label>
  );
}
