import { act, renderHook } from '@testing-library/react';
import {
  MAX_STORED_PER_MESSAGE,
  clearMessageAnnotations,
  getMessageAnnotations,
  handleConceptAnnotationsEvent,
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
    const { result } = renderHook(() => useMessageAnnotations(M1));
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
});
