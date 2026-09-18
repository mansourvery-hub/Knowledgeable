import { act, renderHook } from '@testing-library/react';
import {
  MAX_STORED_ACTIVITIES,
  clearToolProgress,
  getToolProgress,
  handleToolProgressEvent,
  recordToolActivity,
  useToolProgress,
} from '../store/toolProgress';

const M1 = '11111111-1111-1111-1111-111111111111';

function frame(over: Record<string, unknown> = {}) {
  return {
    tool_progress: { phase: 'started', tool_name: 'find_concept', call_id: 'c1' },
    messageId: M1,
    conversationId: 'c1',
    ...over,
  };
}

describe('tool progress store', () => {
  beforeEach(() => {
    clearToolProgress();
  });

  it('stores started then upgrades to finished by call id', () => {
    expect(handleToolProgressEvent(frame())).toBe(true);
    expect(handleToolProgressEvent(frame({ tool_progress: { phase: 'finished', tool_name: 'find_concept', call_id: 'c1' } }))).toBe(
      true,
    );
    const stored = getToolProgress(M1);
    expect(stored).toHaveLength(1);
    expect(stored[0]).toEqual({ tool_name: 'find_concept', call_id: 'c1', phase: 'finished' });
    expect(getToolProgress('missing')).toEqual([]);
  });

  it('rejects malformed envelopes without touching the store', () => {
    for (const bad of [
      null,
      'nope',
      {},
      { messageId: M1 },
      { messageId: '', tool_progress: { phase: 'started', tool_name: 'x', call_id: 'y' } },
      { messageId: M1, tool_progress: 'nope' },
      { messageId: M1, tool_progress: { phase: 'running', tool_name: 'x', call_id: 'y' } },
      { messageId: M1, tool_progress: { tool_name: 'x', call_id: 'y' } },
    ]) {
      expect(handleToolProgressEvent(bad)).toBe(false);
    }
    expect(getToolProgress(M1)).toEqual([]);
  });

  it('caps per-message storage and notifies subscribers', () => {
    const { result } = renderHook(() => useToolProgress(M1));
    act(() => {
      for (let i = 0; i < MAX_STORED_ACTIVITIES + 5; i++) {
        recordToolActivity(M1, { tool_name: `tool-${i}`, call_id: `c${i}`, phase: 'started' });
      }
    });
    expect(result.current).toHaveLength(MAX_STORED_ACTIVITIES);
    act(() => {
      clearToolProgress(M1);
    });
    expect(result.current).toEqual([]);
  });
});
