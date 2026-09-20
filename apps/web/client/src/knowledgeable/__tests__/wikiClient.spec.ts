import { act, renderHook } from '@testing-library/react';
import {
  WikiNotFoundError,
  WikiNotReadyError,
  WikiUnavailableError,
  WikiValidationError,
  WikiError,
  buildWikiUrl,
  fetchMasteredConcepts,
  fetchWikiPage,
  parseMasteredList,
  parseWikiPage,
} from '../api/wikiClient';
import {
  closeWiki,
  getOpenWikiConceptId,
  openWiki,
  useOpenWikiConceptId,
} from '../store/wikiDrawer';

const CONCEPT = '11111111-1111-1111-1111-111111111111';

function stubFetch(
  impl: (url: string) => { status: number; payload: unknown },
) {
  (global.fetch as jest.Mock).mockImplementation((url: string) => {
    const { status, payload } = impl(url);
    return Promise.resolve({
      ok: status >= 200 && status < 300,
      status,
      json: async () => payload,
    } as Response);
  });
}

function pagePayload(over: Record<string, unknown> = {}) {
  return {
    id: 'p1',
    learner_id: 'l1',
    concept_id: CONCEPT,
    title: 'Prime Number',
    summary: 'Numbers with exactly two factors.',
    personalized_content: 'You know factors, so ...',
    known_prerequisites: [],
    related_concepts: [],
    learner_confidence_at_generation: 0.98,
    version: 1,
    is_stale: false,
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-01T00:00:00Z',
    ...over,
  };
}

describe('wikiDrawer store', () => {
  beforeEach(() => {
    closeWiki();
  });

  it('opens, ignores empties, and closes idempotently', () => {
    expect(getOpenWikiConceptId()).toBeNull();
    openWiki('   ');
    expect(getOpenWikiConceptId()).toBeNull();
    const { result } = renderHook(() => useOpenWikiConceptId());
    act(() => {
      openWiki(CONCEPT);
    });
    expect(result.current).toBe(CONCEPT);
    act(() => {
      closeWiki();
    });
    expect(result.current).toBeNull();
  });
});

describe('wikiClient', () => {
  beforeEach(() => {
    global.fetch = jest.fn();
  });

  afterEach(() => {
    jest.resetAllMocks();
  });

  it('builds the wiki URL and rejects empty ids', () => {
    expect(buildWikiUrl(CONCEPT)).toBe(`/api/concepts/${CONCEPT}/wiki`);
    expect(() => buildWikiUrl('  ')).toThrow(WikiValidationError);
  });

  it('returns the typed page on 200', async () => {
    stubFetch(() => ({ status: 200, payload: pagePayload() }));
    const page = await fetchWikiPage(CONCEPT);
    expect(page.title).toBe('Prime Number');
    const url = (global.fetch as jest.Mock).mock.calls[0][0] as string;
    expect(url).toBe(`/api/concepts/${CONCEPT}/wiki`);
  });

  it('maps 404 codes to not-ready vs not-found', async () => {
    stubFetch(() => ({
      status: 404,
      payload: { code: 'wiki_not_ready', message: 'keep learning' },
    }));
    await expect(fetchWikiPage(CONCEPT)).rejects.toBeInstanceOf(WikiNotReadyError);

    (global.fetch as jest.Mock).mockReset();
    stubFetch(() => ({ status: 404, payload: { code: 'not_found', message: 'gone' } }));
    await expect(fetchWikiPage(CONCEPT)).rejects.toBeInstanceOf(WikiNotFoundError);
  });

  it('maps transport failure to unavailable and rejects malformed payloads', async () => {
    (global.fetch as jest.Mock).mockRejectedValue(new TypeError('boom'));
    await expect(fetchWikiPage(CONCEPT)).rejects.toBeInstanceOf(WikiUnavailableError);

    (global.fetch as jest.Mock).mockReset();
    stubFetch(() => ({ status: 200, payload: { nope: true } }));
    await expect(fetchWikiPage(CONCEPT)).rejects.toBeInstanceOf(WikiError);
    expect(parseWikiPage(pagePayload())).toMatchObject({ title: 'Prime Number' });
  });

  it('fetches the mastered list against the W1 contract', async () => {
    stubFetch(() => ({
      status: 200,
      payload: {
        items: [{ id: CONCEPT, name: 'Prime Number', confidence: 0.98, wiki_status: 'ready' }],
        truncated: false,
      },
    }));
    const list = await fetchMasteredConcepts();
    expect(list.items).toHaveLength(1);
    expect(list.items[0]).toMatchObject({ name: 'Prime Number', wiki_status: 'ready' });
    expect(list.truncated).toBe(false);
    const url = (global.fetch as jest.Mock).mock.calls[0][0] as string;
    expect(url).toBe('/api/concepts/mastered?limit=200');
  });

  it('rejects malformed mastered payloads and maps 503 to unavailable', async () => {
    stubFetch(() => ({ status: 200, payload: { items: [{ id: CONCEPT }] } }));
    await expect(fetchMasteredConcepts()).rejects.toBeInstanceOf(WikiError);

    (global.fetch as jest.Mock).mockReset();
    stubFetch(() => ({ status: 503, payload: { message: 'down' } }));
    await expect(fetchMasteredConcepts()).rejects.toBeInstanceOf(WikiUnavailableError);

    expect(
      parseMasteredList({ items: [], truncated: true }),
    ).toMatchObject({ items: [], truncated: true });
  });
});
