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
    if (event.key === 'Enter') { event.preventDefault(); submit(); }
  };
  return (
    <label className="k-field">
      <button
        type="button"
        className="k-field__btn"
        aria-label="Search"
        data-testid={searchButtonTestId(testId)}
        onClick={submit}
        disabled={disabled}
      >
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" aria-hidden="true"><circle cx="11" cy="11" r="7" /><path d="M20 20l-3.5-3.5" /></svg>
      </button>
      <input
        className="k-input"
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={placeholder}
        aria-label={ariaLabel}
        data-testid={testId}
        spellCheck={false}
        disabled={disabled}
        onKeyDown={handleKeyDown}
      />
    </label>
  );
}
