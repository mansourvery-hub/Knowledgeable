/**
 * F16 regression: the header model picker must not mount on the learner
 * surface (only behind `?kdebug`). The picker itself is stubbed — this
 * locks the Header gate, not picker internals.
 */
import React from 'react';
import { render, screen } from '@testing-library/react';
import { RecoilRoot } from 'recoil';
import { MemoryRouter } from 'react-router-dom';
import Header from '../Header';

jest.mock('../Menus/Endpoints/ModelSelector', () => ({
  __esModule: true,
  default: () => <button data-testid="model-selector-button">stub-model</button>,
}));

jest.mock('../Menus', () => ({
  __esModule: true,
  OpenSidebar: () => null,
  PresetsMenu: () => null,
  NewChat: () => null,
  HeaderMenu: () => null,
}));

jest.mock('../TemporaryChat', () => ({
  __esModule: true,
  TemporaryChat: () => null,
  TemporaryChatIndicator: () => null,
}));

jest.mock('../Trace', () => ({
  __esModule: true,
  TraceButton: () => null,
  useTraceControl: () => ({ show: false, open: jest.fn() }),
}));

jest.mock('../ExportAndShareMenu', () => ({
  __esModule: true,
  default: () => null,
}));

jest.mock('../SubagentThreadLink', () => ({
  __esModule: true,
  default: () => null,
}));

jest.mock('../Menus/BookmarkMenu', () => ({
  __esModule: true,
  default: () => null,
}));

jest.mock('../AddMultiConvo', () => ({
  __esModule: true,
  default: () => null,
}));

jest.mock('~/data-provider', () => ({
  useGetStartupConfig: () => ({ data: undefined }),
}));

jest.mock('~/hooks', () => ({
  useHasAccess: () => false,
}));

function setup() {
  render(
    <RecoilRoot>
      <MemoryRouter initialEntries={['/c/new']}>
        <Header />
      </MemoryRouter>
    </RecoilRoot>,
  );
}

describe('Header model picker gate (F16)', () => {
  const originalSearch = window.location.search;

  afterEach(() => {
    window.history.replaceState(null, '', originalSearch || '/');
  });

  it('does not mount the picker on the learner surface', () => {
    window.history.replaceState(null, '', '/c/new');
    setup();
    expect(screen.queryByTestId('model-selector-button')).not.toBeInTheDocument();
  });

  it('mounts behind the ?kdebug escape', () => {
    window.history.replaceState(null, '', '/c/new?kdebug');
    setup();
    expect(screen.getByTestId('model-selector-button')).toBeInTheDocument();
  });
});
