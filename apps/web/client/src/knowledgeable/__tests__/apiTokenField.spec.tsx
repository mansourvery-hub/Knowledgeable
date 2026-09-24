import { fireEvent, render, screen } from '@testing-library/react';
import ApiTokenField from '../components/ApiTokenField';
import { API_TOKEN_KEY } from '../apiToken';

describe('ApiTokenField', () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  afterEach(() => {
    window.localStorage.clear();
  });

  it('saves the typed token and reports set status without echoing it', () => {
    render(<ApiTokenField />);
    expect(screen.getByTestId('api-token-status')).toHaveTextContent('No tester token set.');
    fireEvent.change(screen.getByTestId('api-token-input'), { target: { value: 'tok-123' } });
    fireEvent.click(screen.getByTestId('api-token-save'));
    expect(window.localStorage.getItem(API_TOKEN_KEY)).toBe('tok-123');
    expect(screen.getByTestId('api-token-status')).toHaveTextContent('Tester token saved.');
    expect(screen.getByTestId('api-token-status').textContent).not.toContain('tok-123');
    // Draft clears after save so the secret doesn't sit in the field.
    expect(screen.getByTestId('api-token-input')).toHaveValue('');
  });

  it('clears the stored token', () => {
    window.localStorage.setItem(API_TOKEN_KEY, 'tok-123');
    render(<ApiTokenField />);
    expect(screen.getByTestId('api-token-status')).toHaveTextContent('Tester token saved.');
    fireEvent.click(screen.getByTestId('api-token-clear'));
    expect(window.localStorage.getItem(API_TOKEN_KEY)).toBeNull();
    expect(screen.getByTestId('api-token-status')).toHaveTextContent('No tester token set.');
  });
});
