import { useState, useId, useRef, useEffect, type MouseEvent } from 'react';
import { useRecoilValue } from 'recoil';
import * as Ariakit from '@ariakit/react';
import { BookmarkFilledIcon, BookmarkIcon } from '@radix-ui/react-icons';
import { DropdownPopup, TooltipAnchor, Spinner } from '@librechat/client';
import type { FC } from 'react';
import { BookmarkContext } from '~/Providers/BookmarkContext';
import useBookmarkItems from '~/hooks/Chat/useBookmarkItems';
import { useLocalize } from '~/hooks';
import { Constants } from 'librechat-data-provider';
import { cn } from '~/utils';
import store from '~/store';

const BookmarkMenu: FC = () => {
  const localize = useLocalize();
  const menuId = useId();
  const [isMenuOpen, setIsMenuOpen] = useState(false);
  const conversationId = useRecoilValue(store.conversationByIndex(0))?.conversationId ?? '';
  const { show, items, bookmarks, tags, toggleSaved, isLoading, triggerAriaLabel, dialog } =
    useBookmarkItems();
  /** Single click stars (toggle the Saved tag); double click opens the
   * detail menu. The timer tells them apart: a second press inside the
   * window cancels the star and opens the menu instead. */
  const clickTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  /**
   * The menu stays shut unless a double click explicitly allows it: the
   * trigger's click also reaches Ariakit's disclosure, and `preventDefault`
   * alone does not stop it from opening in a real browser. Gate every open
   * transition instead; closing always passes through (which re-arms).
   * Keyboard Enter/Space stars like a single click — keyboard menu access
   * lives in the header overflow menu, which shares these items.
   */
  const allowMenuOpen = useRef(false);
  const handleSetMenuOpen = (open: boolean) => {
    if (open && !allowMenuOpen.current) {
      return;
    }
    if (!open) {
      allowMenuOpen.current = false;
    }
    setIsMenuOpen(open);
  };
  useEffect(
    () => () => {
      if (clickTimer.current !== null) {
        clearTimeout(clickTimer.current);
      }
    },
    [],
  );

  if (!show) {
    return null;
  }

  const isStarred = tags.includes(Constants.SAVED_TAG);

  const handleTriggerClick = (event: MouseEvent) => {
    // Keep the menu shut: this press is a star unless a second follows.
    event.preventDefault();
    if (isLoading) {
      return;
    }
    if (clickTimer.current !== null) {
      clearTimeout(clickTimer.current);
    }
    clickTimer.current = setTimeout(() => {
      clickTimer.current = null;
      toggleSaved();
    }, 280);
  };

  const handleTriggerDoubleClick = () => {
    if (clickTimer.current !== null) {
      clearTimeout(clickTimer.current);
      clickTimer.current = null;
    }
    allowMenuOpen.current = true;
    setIsMenuOpen(true);
  };

  const renderButtonContent = () => {
    if (isLoading) {
      return <Spinner aria-label="Spinner" />;
    }
    if (isStarred) {
      return <BookmarkFilledIcon className="icon-md" aria-hidden="true" />;
    }
    return <BookmarkIcon className="icon-md" aria-hidden="true" />;
  };

  return (
    <BookmarkContext.Provider value={{ bookmarks }}>
      <DropdownPopup
        portal={true}
        menuId={menuId}
        focusLoop={true}
        isOpen={isMenuOpen}
        unmountOnHide={true}
        setIsOpen={handleSetMenuOpen}
        keyPrefix={`${conversationId}-bookmark-`}
        trigger={
          <TooltipAnchor
            description={localize('com_ui_bookmarks_add')}
            render={
              <Ariakit.MenuButton
                id="bookmark-menu-button"
                aria-label={triggerAriaLabel}
                aria-pressed={isStarred}
                onClick={handleTriggerClick}
                onDoubleClick={handleTriggerDoubleClick}
                className={cn(
                  'mt-text-sm flex size-9 flex-shrink-0 items-center justify-center gap-2 rounded-xl border border-border-light bg-presentation text-sm transition-colors duration-200 hover:bg-surface-hover',
                  isMenuOpen ? 'bg-surface-hover' : '',
                )}
                data-testid="bookmark-menu"
              >
                {renderButtonContent()}
              </Ariakit.MenuButton>
            }
          />
        }
        items={items}
      />
      {dialog}
    </BookmarkContext.Provider>
  );
};

export default BookmarkMenu;
