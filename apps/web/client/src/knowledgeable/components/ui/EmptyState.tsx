export interface EmptyStateProps {
  message: string;
  testId?: string;
}

export default function EmptyState({ message, testId }: EmptyStateProps) {
  return (
    <p className="k-empty" data-testid={testId}>
      {message}
    </p>
  );
}
