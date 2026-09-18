import {
  NeighborhoodNotFoundError,
  NeighborhoodUnavailableError,
  NeighborhoodValidationError,
  NeighborhoodError,
  buildNeighborhoodUrl,
  fetchNeighborhood,
  searchConcepts,
} from '../api/graphClient';

const CONCEPT = '11111111-1111-1111-1111-111111111111';

function jsonResponse(payload: unknown, status: number): Response {
  // jsdom has no fetch globals; stub the surface `graphClient` consumes.
  return {
    ok: status >= 200 && status < 300,
    status,
    json: async () => payload,
  } as Response;
}

describe('graphClient', () => {
  beforeEach(() => {
    global.fetch = jest.fn();
  });

  afterEach(() => {
    jest.resetAllMocks();
  });

  it('builds the /api alias URL with clamped depth/limit', () => {
    expect(buildNeighborhoodUrl(CONCEPT, 2, 10)).toBe(
      `/api/graph/neighborhood?concept_id=${CONCEPT}&depth=2&limit=10`,
    );
    // Out-of-range inputs clamp instead of producing a backend 400.
    expect(buildNeighborhoodUrl(CONCEPT, 255, 0)).toBe(
      `/api/graph/neighborhood?concept_id=${CONCEPT}&depth=5&limit=1`,
    );
  });

  it('rejects an empty concept id without fetching', async () => {
    await expect(fetchNeighborhood({ conceptId: '   ' })).rejects.toBeInstanceOf(
      NeighborhoodValidationError,
    );
    expect(global.fetch).not.toHaveBeenCalled();
  });

  it('returns the typed neighborhood on 200', async () => {
    const payload = { nodes: [], edges: [] };
    (global.fetch as jest.Mock).mockResolvedValue(jsonResponse(payload, 200));
    await expect(fetchNeighborhood({ conceptId: CONCEPT })).resolves.toEqual(payload);
    const url = (global.fetch as jest.Mock).mock.calls[0][0] as string;
    expect(url.startsWith('/api/graph/neighborhood?concept_id=')).toBe(true);
  });

  it('maps 400 to validation, 404 to not-found, 503 to unavailable', async () => {
    const fetchMock = global.fetch as jest.Mock;

    fetchMock.mockResolvedValue(jsonResponse({ code: 'validation_failed', message: 'bad uuid' }, 400));
    await expect(fetchNeighborhood({ conceptId: CONCEPT })).rejects.toBeInstanceOf(
      NeighborhoodValidationError,
    );

    fetchMock.mockResolvedValue(jsonResponse({ code: 'not_found', message: 'gone' }, 404));
    await expect(fetchNeighborhood({ conceptId: CONCEPT })).rejects.toBeInstanceOf(
      NeighborhoodNotFoundError,
    );

    fetchMock.mockResolvedValue(
      jsonResponse({ code: 'service_unavailable', message: 'db down' }, 503),
    );
    await expect(fetchNeighborhood({ conceptId: CONCEPT })).rejects.toBeInstanceOf(
      NeighborhoodUnavailableError,
    );
  });

  it('maps other statuses to NeighborhoodError with the envelope code', async () => {
    (global.fetch as jest.Mock).mockResolvedValue(
      jsonResponse({ code: 'rate_limited', message: 'slow down' }, 429),
    );
    const err = await fetchNeighborhood({ conceptId: CONCEPT }).catch((e) => e);
    expect(err).toBeInstanceOf(NeighborhoodError);
    expect((err as NeighborhoodError).code).toBe('rate_limited');
  });

  it('maps network failure to unavailable (never a raw TypeError leak)', async () => {
    (global.fetch as jest.Mock).mockRejectedValue(new TypeError('boom'));
    await expect(fetchNeighborhood({ conceptId: CONCEPT })).rejects.toBeInstanceOf(
      NeighborhoodUnavailableError,
    );
  });

  it('rejects malformed payloads instead of crashing render', async () => {
    (global.fetch as jest.Mock).mockResolvedValue(jsonResponse({ nope: true }, 200));
    await expect(fetchNeighborhood({ conceptId: CONCEPT })).rejects.toBeInstanceOf(
      NeighborhoodError,
    );
  });
});

describe('searchConcepts', () => {
  beforeEach(() => {
    global.fetch = jest.fn();
  });

  afterEach(() => {
    jest.resetAllMocks();
  });

  it('short-circuits empty queries without fetching', async () => {
    await expect(searchConcepts('   ')).resolves.toEqual([]);
    expect(global.fetch).not.toHaveBeenCalled();
  });

  it('returns sanitized hits and drops invalid entries', async () => {
    (global.fetch as jest.Mock).mockResolvedValue(
      jsonResponse(
        [
          { id: 'a', canonical_name: 'Alpha', canonical_statement: 'S.' },
          { id: '', canonical_name: 'Bad', canonical_statement: 'S.' },
          { nope: true },
        ],
        200,
      ),
    );
    const hits = await searchConcepts('alpha');
    expect(hits).toEqual([{ id: 'a', canonical_name: 'Alpha', canonical_statement: 'S.' }]);
    const url = (global.fetch as jest.Mock).mock.calls[0][0] as string;
    expect(url.startsWith('/api/concepts/search?')).toBe(true);
  });

  it('maps transport failure to unavailable', async () => {
    (global.fetch as jest.Mock).mockRejectedValue(new TypeError('boom'));
    await expect(searchConcepts('alpha')).rejects.toBeInstanceOf(
      NeighborhoodUnavailableError,
    );
  });
});
