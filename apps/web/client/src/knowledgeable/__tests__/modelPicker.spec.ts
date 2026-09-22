/**
 * F16 contract: the header model picker is hidden from learners and mounts
 * only behind the `?kdebug` escape (or when the flag flips for BYOK).
 */
import { modelPickerDisabled, showModelPicker } from '../modelPicker';

describe('showModelPicker', () => {
  it('is disabled by default', () => {
    expect(modelPickerDisabled).toBe(true);
  });

  it('hides the picker from the learner surface', () => {
    expect(showModelPicker('')).toBe(false);
    expect(showModelPicker('?x=1')).toBe(false);
  });

  it('mounts behind the ?kdebug escape', () => {
    expect(showModelPicker('?kdebug')).toBe(true);
    expect(showModelPicker('?kdebug=1&x=2')).toBe(true);
  });
});
