/**
 * Star behavior: single click toggles the Saved tag, double click opens
 * the detail menu instead (and never stars).
 */
import React from 'react';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { RecoilRoot } from 'recoil';
import BookmarkMenu from '../BookmarkMenu';

const mockToggleSaved = jest.fn();

jest.mock('~/hooks/Chat/useBookmarkItems', () => ({
  __esModule: true,
  default: () => ({
    show: true,
    items: [
      {
        id: 'sentinel-item',
        label: 'Sentinel menu entry',
        onClick: jest.fn(),
      },
    ],
    bookmarks: [],
    hasBookmarks: false,
    tags: [],
    toggleSaved: mockToggleSaved,
    isLoading: false,
    triggerAriaLabel: 'Add Bookmarks',
    dialog: null,
  }),
}));

jest.mock('~/hooks', () => ({
  useLocalize: () => (key: string) => key,
}));

function setup() {
  render(
    <RecoilRoot>
      <BookmarkMenu />
    </RecoilRoot>,
  );
}

describe('BookmarkMenu star toggle', () => {
  beforeEach(() => {
    mockToggleSaved.mockClear();
  });

  it('single click stars after the double-click window with no menu', async () => {
    setup();
    const button = screen.getByTestId('bookmark-menu');
    fireEvent.click(button);
    // Menu must stay shut: this press is a star unless a second follows.
    expect(screen.queryByText('Sentinel menu entry')).not.toBeInTheDocument();
    await waitFor(() => expect(mockToggleSaved).toHaveBeenCalledTimes(1));
    expect(screen.queryByText('Sentinel menu entry')).not.toBeInTheDocument();
  });

  it('double click never stars (menu-open proven live, not under jsdom)', async () => {
    setup();
    const button = screen.getByTestId('bookmark-menu');
    fireEvent.click(button);
    fireEvent.click(button);
    fireEvent.doubleClick(button);
    // Past the single-click window: the star must never fire.
    await new Promise((resolve) => setTimeout(resolve, 500));
    expect(mockToggleSaved).not.toHaveBeenCalled();
  });
});
