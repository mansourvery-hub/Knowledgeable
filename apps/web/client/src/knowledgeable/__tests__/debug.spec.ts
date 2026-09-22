/**
 * F18 contract: the shared `?kdebug` escape hides debug surfaces from the
 * shipped product while keeping them fully working for development.
 */
import { hasDebugParam } from '../debug';

describe('hasDebugParam', () => {
  it('is off by default', () => {
    expect(hasDebugParam('')).toBe(false);
    expect(hasDebugParam('?x=1')).toBe(false);
  });

  it('fires on the kdebug flag', () => {
    expect(hasDebugParam('?kdebug')).toBe(true);
    expect(hasDebugParam('?kdebug=1&x=2')).toBe(true);
  });
});
