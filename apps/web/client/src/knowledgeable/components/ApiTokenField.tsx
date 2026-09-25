/**
 * Tester API token field (`?kdebug` settings entry).
 *
 * Password input + Save/Clear against localStorage (`knowledgeable.apiToken`).
 * Reports set/unset status only — the value is never rendered back, never
 * logged. Takes effect immediately (the axios interceptor reads per request;
 * new SSE submissions read at submit time); no reload needed.
 */
import { useState } from 'react';
import { getApiToken, setApiToken, syncDefaultToken } from '../apiToken';
import Button from './ui/Button';

export default function ApiTokenField() {
  const [draft, setDraft] = useState('');
  const [showToken, setShowToken] = useState(false);
  const [saved, setSaved] = useState(() => getApiToken() !== '');

  const onSave = () => {
    setApiToken(draft);
    setDraft('');
    syncDefaultToken();
    setSaved(getApiToken() !== '');
  };
  const onClear = () => {
    setApiToken('');
    setDraft('');
    syncDefaultToken();
    setSaved(false);
  };

  return (
    <div data-testid="api-token-field" className="flex flex-col gap-2">
      <div className="k-field">
        <input
          type={showToken ? 'text' : 'password'}
          className="k-input"
          value={draft}
          onChange={(event) => setDraft(event.target.value)}
          placeholder="Paste tester token"
          aria-label="Tester API token"
          data-testid="api-token-input"
          autoComplete="off"
          spellCheck={false}
          onKeyDown={(event) => {
            if (event.key === 'Enter') {
              event.preventDefault();
              onSave();
            }
          }}
        />
        {draft && (
          <button
            type="button"
            className="k-field__clear"
            aria-label={showToken ? 'Hide token' : 'Show token'}
            onClick={() => setShowToken(!showToken)}
          >
            {showToken ? (
              <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24" />
                <line x1="1" y1="1" x2="23" y2="23" />
              </svg>
            ) : (
              <svg width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
                <circle cx="12" cy="12" r="3" />
              </svg>
            )}
          </button>
        )}
      </div>
      <div className="flex gap-2">
        <Button variant="primary" onClick={onSave} testId="api-token-save" ariaLabel="Save tester API token">
          Save
        </Button>
        <Button variant="ghost" onClick={onClear} testId="api-token-clear" ariaLabel="Clear tester API token">
          Clear
        </Button>
      </div>
      <p data-testid="api-token-status" className="k-stale">
        {saved ? 'Tester token saved.' : 'No tester token set.'}
      </p>
    </div>
  );
}
