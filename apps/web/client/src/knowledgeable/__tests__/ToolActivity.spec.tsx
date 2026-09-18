import { act, render, screen } from '@testing-library/react';
import ToolActivity, { describeToolActivity } from '../components/ToolActivity';
import { clearToolProgress, recordToolActivity } from '../store/toolProgress';

const M1 = '11111111-1111-1111-1111-111111111111';

describe('ToolActivity', () => {
  beforeEach(() => {
    clearToolProgress();
  });

  it('renders nothing without activities', () => {
    render(<ToolActivity messageId={M1} />);
    expect(screen.queryByTestId('tool-activity')).not.toBeInTheDocument();
  });

  it('shows pulsing active checks and settled finished ones', () => {
    render(<ToolActivity messageId={M1} />);
    act(() => {
      recordToolActivity(M1, {
        tool_name: 'get_weak_dependencies',
        call_id: 'c1',
        phase: 'started',
      });
    });
    const item = screen.getByTestId('tool-activity-item');
    expect(item).toHaveAttribute('data-phase', 'started');
    expect(item).toHaveClass('animate-pulse');
    expect(item).toHaveTextContent('Checking weak prerequisites…');

    act(() => {
      recordToolActivity(M1, {
        tool_name: 'get_weak_dependencies',
        call_id: 'c1',
        phase: 'finished',
      });
    });
    const settled = screen.getByTestId('tool-activity-item');
    expect(settled).toHaveAttribute('data-phase', 'finished');
    expect(settled).not.toHaveClass('animate-pulse');
    expect(settled).toHaveTextContent('✓');
  });

  it('falls back to the raw tool name for unknown tools', () => {
    expect(describeToolActivity('mystery_tool', 'started')).toBe('mystery_tool…');
  });
});
