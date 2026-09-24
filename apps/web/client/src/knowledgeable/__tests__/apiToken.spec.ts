import axios from 'axios';
import {
  API_TOKEN_KEY,
  clearApiToken,
  getApiToken,
  hasApiToken,
  resolveBearerToken,
  setApiToken,
  syncDefaultToken,
  withApiTokenHeader,
} from '../apiToken';

describe('apiToken', () => {
  beforeEach(() => {
    window.localStorage.clear();
    window.history.replaceState(null, '', '/c/new');
  });

  afterEach(() => {
    window.localStorage.clear();
    window.history.replaceState(null, '', '/c/new');
  });

  it('starts empty', () => {
    expect(getApiToken()).toBe('');
    expect(hasApiToken()).toBe(false);
  });

  it('round-trips set/clear without echoing the value', () => {
    setApiToken('  secret-1  ');
    expect(getApiToken()).toBe('secret-1');
    expect(hasApiToken()).toBe(true);
    expect(window.localStorage.getItem(API_TOKEN_KEY)).toBe('secret-1');
    clearApiToken();
    expect(getApiToken()).toBe('');
  });

  it('captures ?ktoken= once, persists it, and strips it from the URL', () => {
    expect(getApiToken('?ktoken=capture-me')).toBe('capture-me');
    expect(window.localStorage.getItem(API_TOKEN_KEY)).toBe('capture-me');
    expect(window.location.search).not.toContain('ktoken');
    // Second read comes from storage even without the param.
    expect(getApiToken('')).toBe('capture-me');
  });

  it('passes the session token through, stored token wins', () => {
    expect(resolveBearerToken('local-session-token')).toBe('local-session-token');
    expect(resolveBearerToken(undefined)).toBe('');
    setApiToken('stored-1');
    expect(resolveBearerToken('local-session-token')).toBe('stored-1');
    expect(resolveBearerToken(undefined)).toBe('stored-1');
  });

  it('header rule overwrites only when a token is stored', () => {
    expect(withApiTokenHeader(undefined, '')).toBeUndefined();
    expect(withApiTokenHeader('Bearer local-session-token', '')).toBe('Bearer local-session-token');
    expect(withApiTokenHeader(undefined, 'stored-1')).toBe('Bearer stored-1');
    expect(withApiTokenHeader('Bearer local-session-token', 'stored-1')).toBe('Bearer stored-1');
  });

  it('syncDefaultToken mirrors storage into the axios default', () => {
    delete axios.defaults.headers.common['Authorization'];
    setApiToken('stored-1');
    syncDefaultToken();
    expect(axios.defaults.headers.common['Authorization']).toBe('Bearer stored-1');
    clearApiToken();
    syncDefaultToken();
    expect(axios.defaults.headers.common['Authorization']).toBeUndefined();
  });
});
