/**
 * F13 regression: the Folders list "refreshed uncontrollably every few
 * seconds". Root cause was `useProjectsInfiniteQuery` inheriting React
 * Query's `refetchOnWindowFocus`/`refetchOnReconnect` defaults while no
 * `/api/projects*` backend exists — every focus/reconnect fired a fetch
 * that 404ed into a full retry storm with spinner flicker. The hook now
 * disables focus/reconnect refetches by default (mount fetches still run).
 *
 * Both tests mock the missing backend as a rejection: a permanently
 * failing query never holds fresh data, so EVERY focus/reconnect event
 * refetches pre-fix — exactly the reported behavior.
 */
import {
  QueryClient,
  QueryClientProvider,
  focusManager,
  onlineManager,
} from '@tanstack/react-query';
import { renderHook, waitFor } from '@testing-library/react';
import React from 'react';

const mockListProjects = jest.fn();
jest.mock('librechat-data-provider', () => {
  const actual = jest.requireActual('librechat-data-provider');
  return {
    ...actual,
    dataService: {
      ...actual.dataService,
      listProjects: (...args: unknown[]) => mockListProjects(...args),
    },
  };
});

import { useProjectsInfiniteQuery } from '../queries';

function wrapper(queryClient: QueryClient) {
  return function Wrapper({ children }: { children: React.ReactNode }) {
    return <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>;
  };
}

describe('useProjectsInfiniteQuery focus/reconnect refetch (F13)', () => {
  let queryClient: QueryClient;

  beforeEach(() => {
    mockListProjects.mockReset();
    // The missing backend: every fetch rejects (live: JSON 404).
    mockListProjects.mockRejectedValue(new Error('404'));
    // retry: false isolates the event-driven refetch from the retry storm,
    // which is correct-with-a-backend behavior this brick does not change.
    queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
      logger: { log: () => {}, warn: () => {}, error: () => {} },
    });
  });

  afterEach(() => {
    queryClient.clear();
    focusManager.setFocused(true);
    onlineManager.setOnline(true);
  });

  it('fetches once on mount and ignores window focus', async () => {
    renderHook(() => useProjectsInfiniteQuery({ limit: 25 }), {
      wrapper: wrapper(queryClient),
    });

    await waitFor(() => {
      expect(mockListProjects).toHaveBeenCalledTimes(1);
    });

    window.dispatchEvent(new Event('focus'));
    document.dispatchEvent(new Event('visibilitychange'));
    // Let any scheduled refetch land, then confirm still exactly one fetch.
    await new Promise((resolve) => setTimeout(resolve, 500));
    expect(mockListProjects).toHaveBeenCalledTimes(1);
  });

  it('ignores reconnect events', async () => {
    renderHook(() => useProjectsInfiniteQuery({ limit: 25 }), {
      wrapper: wrapper(queryClient),
    });

    await waitFor(() => {
      expect(mockListProjects).toHaveBeenCalledTimes(1);
    });

    window.dispatchEvent(new Event('online'));
    await new Promise((resolve) => setTimeout(resolve, 500));
    expect(mockListProjects).toHaveBeenCalledTimes(1);
  });
});
