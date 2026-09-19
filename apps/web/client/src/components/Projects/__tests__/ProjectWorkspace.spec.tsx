import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import { createMemoryRouter, RouterProvider } from 'react-router-dom';
import ProjectWorkspace from '../ProjectWorkspace';

jest.mock('~/hooks', () => ({
  useLocalize: () => (key: string) => key,
  useNewConvo: () => ({ newConversation: jest.fn() }),
}));

jest.mock('~/data-provider', () => ({
  useProjectQuery: jest.fn(() => ({ data: undefined, isLoading: false })),
  useConversationsInfiniteQuery: jest.fn(() => ({
    data: undefined,
    fetchNextPage: jest.fn(),
    isFetchingNextPage: false,
    isLoading: false,
  })),
}));

jest.mock('recoil', () => {
  const actual = jest.requireActual('recoil');
  return { ...actual, useRecoilValue: jest.fn(() => null) };
});

jest.mock('@tanstack/react-query', () => ({
  useQueryClient: jest.fn(() => ({})),
}));

jest.mock('@librechat/client', () => {
  const actual = jest.requireActual('@librechat/client');
  return { ...actual, useMediaQuery: jest.fn(() => false) };
});

function setup(path: string) {
  const router = createMemoryRouter(
    [
      { path: '/projects/:projectId', element: <ProjectWorkspace /> },
      { path: '/c/new', element: <div data-testid="chat" /> },
    ],
    { initialEntries: [path] },
  );
  render(<RouterProvider router={router} />);
  return router;
}

describe('ProjectWorkspace (Knowledgeable: projects surface disabled)', () => {
  it('redirects /projects/:id to new chat instead of rendering a dead workspace', async () => {
    const router = setup('/projects/some-id');
    await waitFor(() => expect(router.state.location.pathname).toBe('/c/new'));
    expect(screen.getByTestId('chat')).toBeInTheDocument();
  });
});
