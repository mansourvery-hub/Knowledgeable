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
      <svg
        width="18"
        height="18"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeLinecap="round"
        strokeLinejoin="round"
        aria-hidden="true"
        style={{ flexShrink: 0 }}
      >
        <circle cx="12" cy="12" r="10" />
        <line x1="12" y1="8" x2="12" y2="12" />
        <line x1="12" y1="16" x2="12.01" y2="16" />
      </svg>
      <span>{message}</span>
      {onRetry && (
        <Button onClick={onRetry} testId={retryTestId}>
          {retryLabel}
        </Button>
      )}
    </div>
  );
}
