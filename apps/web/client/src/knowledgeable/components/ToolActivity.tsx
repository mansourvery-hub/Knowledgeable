/**
 * Inline tutor-activity indicator for M4 tool progress (T14, brick C2).
 *
 * Renders beneath the assistant markdown as a subtle, non-interactive status
 * list: active graph checks pulse gently, finished ones settle to a check.
 * Purely presentational over `store/toolProgress.ts`; empty when the turn
 * used no tools, so plain chat renders exactly as before.
 */
import { useToolProgress } from '../store/toolProgress';

const TOOL_VERBS: Record<string, string> = {
  find_concept: 'Looking up concept',
  get_concept: 'Reading concept',
  get_dependencies: 'Checking prerequisites',
  get_weak_dependencies: 'Checking weak prerequisites',
  get_related_concepts: 'Finding related concepts',
  log_observation: 'Noting observation',
  propose_concept: 'Proposing concept',
  propose_relation: 'Proposing relation',
};

export function describeToolActivity(toolName: string, phase: 'started' | 'finished'): string {
  const verb = TOOL_VERBS[toolName] ?? toolName;
  return phase === 'started' ? `${verb}…` : `${verb} ✓`;
}

export default function ToolActivity({ messageId }: { messageId?: string | null }) {
  const activities = useToolProgress(messageId);
  if (activities.length === 0) {
    return null;
  }
  return (
    <div
      data-testid="tool-activity"
      aria-live="polite"
      className="mt-1 flex flex-col gap-0.5 text-xs text-text-secondary"
    >
      {activities.map((activity) => (
        <span
          key={activity.call_id}
          data-testid="tool-activity-item"
          data-phase={activity.phase}
          data-tool={activity.tool_name}
          className={activity.phase === 'started' ? 'animate-pulse' : undefined}
        >
          {describeToolActivity(activity.tool_name, activity.phase)}
        </span>
      ))}
    </div>
  );
}
