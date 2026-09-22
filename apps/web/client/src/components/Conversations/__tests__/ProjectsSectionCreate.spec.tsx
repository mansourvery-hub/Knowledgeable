/**
 * F14 regression: folder creation has no backend contract (`POST
 * /api/projects` 404s), so the sidebar empty state must be static copy —
 * never a "New folder" button opening a dialog that can only fail.
 */
import React from 'react';
import { render, screen } from '@testing-library/react';
import { RecoilRoot } from 'recoil';
import { createMemoryRouter, RouterProvider } from 'react-router-dom';
import ProjectsSection from '../ProjectsSection';

jest.mock('~/hooks', () => ({
  useLocalize: () => (key: string) => key,
  useLocalStorage: () => [true, jest.fn()],
  useNewConvo: () => ({ newConversation: jest.fn() }),
}));

jest.mock('~/data-provider', () => ({
  useProjectsInfiniteQuery: jest.fn(() => ({
    data: { pages: [{ projects: [], nextCursor: null }] },
    fetchNextPage: jest.fn(),
    isFetchingNextPage: false,
    isLoading: false,
    isFetching: false,
    isError: false,
  })),
  useActiveJobs: jest.fn(() => ({})),
  useConversationsInfiniteQuery: jest.fn(() => ({
    data: { pages: [] },
    isLoading: false,
  })),
  // The unopenable create dialog still mounts and calls this hook.
  useCreateProjectMutation: jest.fn(() => ({ mutateAsync: jest.fn(), isLoading: false })),
}));

function setup() {
  const router = createMemoryRouter(
    [{ path: '/c/new', element: <ProjectsSection toggleNav={() => {}} isAuthenticated={true} /> }],
    { initialEntries: ['/c/new'] },
  );
  render(
    <RecoilRoot>
      <RouterProvider router={router} />
    </RecoilRoot>,
  );
}

describe('ProjectsSection empty state (F14: no dead create button)', () => {
  it('renders static copy with no create affordance', () => {
    setup();
    expect(screen.getByText('com_ui_no_projects')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'com_ui_new_project' })).not.toBeInTheDocument();
    expect(screen.queryByText('com_ui_create_project')).not.toBeInTheDocument();
  });
});
