import Button from './Button';

export interface ErrorStateProps {
  message: string;
  onRetry?: () => void;
  retryLabel?: string;
  testId?: string;
  retryTestId?: string;
}

export default function ErrorState({
  message,
  onRetry,
  retryLabel = 'Try again',
  testId,
  retryTestId,
}: ErrorStateProps) {
  return (
    <div className="k-error" role="alert" data-testid={testId}>
      <span>{message}</span>
      {onRetry && (
        <Button onClick={onRetry} testId={retryTestId}>
          {retryLabel}
        </Button>
      )}
    </div>
  );
}
