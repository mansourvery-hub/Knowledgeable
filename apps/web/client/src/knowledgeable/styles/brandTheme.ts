/**
 * Knowledgeable brand palette as an upstream `ThemeDefinition` (Phase 4, SPEC 8.2).
 *
 * The runtime `ThemeProvider` writes its variables as inline styles on
 * `<html>`, so a stylesheet override (`brand.css`) would lose to it. The
 * palette therefore travels through the provider's `themeDefinition` prop
 * (one seam in `App.jsx`) instead. `resolveTheme` merges these roles over
 * the per-mode bases; every other token keeps its upstream value.
 *
 * Values are RGB channel triplets matching the upstream format
 * (`docs/agent-context/ui-redesign-notes.md` §1).
 */
import type { ThemeDefinition } from '@librechat/client';

const light = {
  'rgb-surface-primary': '244 246 245',
  'rgb-surface-secondary': '235 239 237',
  'rgb-surface-tertiary': '255 255 255',
  'rgb-border-light': '213 221 224',
  'rgb-text-primary': '20 33 43',
  'rgb-text-secondary': '74 91 104',
  'rgb-text-tertiary': '90 107 119',
};

const dark = {
  'rgb-surface-primary': '17 29 39',
  'rgb-surface-secondary': '13 23 32',
  'rgb-surface-tertiary': '24 39 51',
  'rgb-border-light': '36 55 70',
  'rgb-text-primary': '230 237 242',
  'rgb-text-secondary': '163 180 194',
  'rgb-text-tertiary': '124 143 160',
};

export const brandThemeDefinition: ThemeDefinition = {
  version: 1,
  name: 'knowledgeable',
  modes: {
    light: { colors: light },
    dark: { colors: dark },
  },
};
