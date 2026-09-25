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

  it('saves token on Enter key and toggles password visibility', () => {
    render(<ApiTokenField />);
    const input = screen.getByTestId('api-token-input');
    expect(input).toHaveAttribute('type', 'password');

    fireEvent.change(input, { target: { value: 'tok-enter' } });
    const toggleBtn = screen.getByRole('button', { name: 'Show token' });
    fireEvent.click(toggleBtn);
    expect(input).toHaveAttribute('type', 'text');

    fireEvent.keyDown(input, { key: 'Enter' });
    expect(window.localStorage.getItem(API_TOKEN_KEY)).toBe('tok-enter');
    expect(screen.getByTestId('api-token-status')).toHaveTextContent('Tester token saved.');
  });
});
