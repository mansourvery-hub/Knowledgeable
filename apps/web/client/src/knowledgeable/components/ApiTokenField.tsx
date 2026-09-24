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
      <input
        type="password"
        className="k-input"
        value={draft}
        onChange={(event) => setDraft(event.target.value)}
        placeholder="Paste tester token"
        aria-label="Tester API token"
        data-testid="api-token-input"
        autoComplete="off"
        spellCheck={false}
      />
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
