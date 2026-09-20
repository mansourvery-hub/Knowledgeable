import type { ReactNode } from 'react';
import { act, renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { QueryKeys } from 'librechat-data-provider';
import {
  MAX_STORED_PER_MESSAGE,
  clearMessageAnnotations,
  getMessageAnnotations,
  handleConceptAnnotationsEvent,
  sanitizeAnnotations,
  setMessageAnnotations,
  useMessageAnnotations,
} from '../store/annotations';

const M1 = '11111111-1111-1111-1111-111111111111';

function frame(over: Record<string, unknown> = {}) {
  return {
    concept_annotations: [
      {
        concept_id: 'aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa',
        name: 'Prime Number',
        learner_confidence: 0.98,
        status: 'known',
      },
      {
        concept_id: 'bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb',
        name: 'Factor',
        learner_confidence: 0.3,
        status: 'weak',
      },
    ],
    messageId: M1,
    conversationId: 'c1',
    ...over,
  };
}

describe('annotation store', () => {
  beforeEach(() => {
    clearMessageAnnotations();
  });

  it('stores and reads annotations per message', () => {
    expect(handleConceptAnnotationsEvent(frame())).toBe(true);
    const stored = getMessageAnnotations(M1);
    expect(stored).toHaveLength(2);
    expect(stored[0]).toMatchObject({ name: 'Prime Number', status: 'known' });
    expect(getMessageAnnotations('missing')).toEqual([]);
    expect(getMessageAnnotations(undefined)).toEqual([]);
  });

  it('rejects malformed envelopes without touching the store', () => {
    for (const bad of [
      null,
      'nope',
      {},
      { messageId: M1 },
      { messageId: '', concept_annotations: [] },
      { messageId: M1, concept_annotations: 'nope' },
    ]) {
      expect(handleConceptAnnotationsEvent(bad)).toBe(false);
    }
    expect(getMessageAnnotations(M1)).toEqual([]);
  });

  it('consumes a well-formed envelope even when no entry survives', () => {
    expect(
      handleConceptAnnotationsEvent({
        messageId: M1,
        concept_annotations: [{ name: 'x' }],
      }),
    ).toBe(true);
    expect(getMessageAnnotations(M1)).toEqual([]);
  });

  it('sanitizes entries and caps per-message storage', () => {
    const many = Array.from({ length: MAX_STORED_PER_MESSAGE + 5 }, (_, i) => ({
      concept_id: `id-${i}`,
      name: `Concept ${i}`,
      learner_confidence: Number.NaN,
      status: 'new',
    }));
    expect(
      handleConceptAnnotationsEvent(frame({ concept_annotations: many })),
    ).toBe(true);
    const stored = getMessageAnnotations(M1);
    expect(stored).toHaveLength(MAX_STORED_PER_MESSAGE);
    // NaN confidence degrades to null, never poisons render.
    expect(stored[0].learner_confidence).toBeNull();
    expect(stored[0].status).toBe('new');
  });

  it('notifies hook subscribers and supports targeted/all clears', () => {
    const queryClient = new QueryClient();
    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );
    const { result } = renderHook(() => useMessageAnnotations(M1), { wrapper });
    expect(result.current).toEqual([]);
    act(() => {
      setMessageAnnotations(M1, [
        {
          concept_id: 'a',
          name: 'Alpha',
          learner_confidence: null,
          status: 'new',
        },
      ]);
    });
    expect(result.current).toHaveLength(1);
    act(() => {
      clearMessageAnnotations(M1);
    });
    expect(result.current).toEqual([]);
  });

  describe('history hydration (F7 Brick 2)', () => {
    const M2 = '22222222-2222-2222-2222-222222222222';
    let queryClient: QueryClient;
    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );

    beforeEach(() => {
      queryClient = new QueryClient();
    });

    function seedHistory(messages: unknown[]) {
      queryClient.setQueryData([QueryKeys.messages, 'c1'], messages);
    }

    it('adopts embedded annotations for unstored ids', async () => {
      seedHistory([
        {
          messageId: M2,
          concept_annotations: [
            { concept_id: 'a', name: 'Alpha', learner_confidence: 0.9, status: 'known' },
          ],
        },
      ]);
      const { result } = renderHook(() => useMessageAnnotations(M2), { wrapper });
      await waitFor(() => expect(result.current).toHaveLength(1));
      expect(result.current[0]).toMatchObject({ name: 'Alpha', status: 'known' });
    });

    it('prefers live store entries over history', () => {
      setMessageAnnotations(M2, [
        { concept_id: 'b', name: 'Beta', learner_confidence: null, status: 'new' },
      ]);
      seedHistory([
        {
          messageId: M2,
          concept_annotations: [
            { concept_id: 'a', name: 'Alpha', learner_confidence: 0.9, status: 'known' },
          ],
        },
      ]);
      const { result } = renderHook(() => useMessageAnnotations(M2), { wrapper });
      expect(result.current).toHaveLength(1);
      expect(result.current[0]).toMatchObject({ name: 'Beta' });
    });

    it('ignores malformed embedded payloads and empty caches', async () => {
      seedHistory([{ messageId: M2, concept_annotations: [{ nope: true }] }]);
      const { result } = renderHook(() => useMessageAnnotations(M2), { wrapper });
      expect(result.current).toEqual([]);

      clearMessageAnnotations();
      const idle = renderHook(() => useMessageAnnotations(M2), { wrapper });
      expect(idle.result.current).toEqual([]);
    });

    it('sanitizes raw payloads for render consumers', () => {
      expect(
        sanitizeAnnotations([
          { concept_id: 'a', name: 'Alpha', learner_confidence: 0.5, status: 'known' },
          { nope: true },
        ]),
      ).toMatchObject([{ name: 'Alpha', status: 'known' }]);
      expect(sanitizeAnnotations('nope')).toEqual([]);
      expect(sanitizeAnnotations(undefined)).toEqual([]);
    });
  });
});
