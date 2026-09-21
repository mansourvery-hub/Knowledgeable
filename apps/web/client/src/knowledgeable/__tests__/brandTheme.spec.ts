/**
 * Brand palette contract (Phase 4, SPEC 8.2).
 *
 * The definition must validate against the upstream theme registry, resolve
 * to our triplets over the per-mode bases, and keep every text-on-surface
 * pairing at 4.5:1 or better in both modes.
 */
import { resolveTheme, validateThemeDefinition } from '@librechat/client';
import { brandThemeDefinition } from '../styles/brandTheme';

const ROLES = [
  'rgb-surface-primary',
  'rgb-surface-secondary',
  'rgb-surface-tertiary',
  'rgb-border-light',
  'rgb-text-primary',
  'rgb-text-secondary',
  'rgb-text-tertiary',
] as const;

function luminance(triplet: string): number {
  const channel = (v: number): number => {
    const s = v / 255;
    return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
  };
  const [r, g, b] = triplet.split(' ').map(Number).map(channel);
  return 0.2126 * (r as number) + 0.7152 * (g as number) + 0.0722 * (b as number);
}

function contrast(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return ((hi as number) + 0.05) / ((lo as number) + 0.05);
}

describe('brandThemeDefinition', () => {
  it('validates cleanly against the upstream theme registry', () => {
    expect(validateThemeDefinition(brandThemeDefinition)).toEqual([]);
  });

  it('resolves the seven role overrides over the per-mode bases', () => {
    for (const mode of ['light', 'dark'] as const) {
      const resolved = resolveTheme(brandThemeDefinition, mode);
      const expected = brandThemeDefinition.modes[mode]?.colors ?? {};
      for (const role of ROLES) {
        expect(resolved.colors[role]).toBe(expected[role]);
      }
      // Untouched tokens keep their upstream values.
      expect(resolved.colors['rgb-surface-submit']).toBeTruthy();
    }
  });

  it('keeps text-on-surface contrast at 4.5:1 or better in both modes', () => {
    for (const mode of ['light', 'dark'] as const) {
      const colors = resolveTheme(brandThemeDefinition, mode).colors;
      const texts = ['rgb-text-primary', 'rgb-text-secondary', 'rgb-text-tertiary'] as const;
      const surfaces = ['rgb-surface-primary', 'rgb-surface-secondary', 'rgb-surface-tertiary'] as const;
      for (const text of texts) {
        for (const surface of surfaces) {
          const ratio = contrast(colors[text], colors[surface]);
          expect(ratio).toBeGreaterThanOrEqual(4.5);
        }
      }
    }
  });
});
