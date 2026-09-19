import React from 'react';
import { render, screen, waitFor } from '@testing-library/react';
import { createMemoryRouter, RouterProvider } from 'react-router-dom';
import ProjectsView from '../ProjectsView';

jest.mock('~/hooks', () => ({
  useLocalize: () => (key: string) => key,
}));

jest.mock('~/data-provider', () => ({
  useProjectsInfiniteQuery: jest.fn(() => ({
    data: undefined,
    fetchNextPage: jest.fn(),
    isFetchingNextPage: false,
    isLoading: false,
  })),
}));

function setup(path: string) {
  const router = createMemoryRouter(
    [
      { path: '/projects', element: <ProjectsView /> },
      { path: '/c/new', element: <div data-testid="chat" /> },
    ],
    { initialEntries: [path] },
  );
  render(<RouterProvider router={router} />);
  return router;
}

describe('ProjectsView (Knowledgeable: projects surface disabled)', () => {
  it('redirects /projects to new chat instead of rendering a dead workspace', async () => {
    const router = setup('/projects');
    await waitFor(() => expect(router.state.location.pathname).toBe('/c/new'));
    expect(screen.getByTestId('chat')).toBeInTheDocument();
  });
});
