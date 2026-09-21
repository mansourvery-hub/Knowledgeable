import type { ReactNode } from 'react';

export interface ButtonProps {
  children: ReactNode;
  variant?: 'primary' | 'ghost' | 'default';
  type?: 'button' | 'submit';
  disabled?: boolean;
  onClick?: () => void;
  testId?: string;
  ariaLabel?: string;
}

export default function Button({
  children,
  variant = 'default',
  type = 'button',
  disabled,
  onClick,
  testId,
  ariaLabel,
}: ButtonProps) {
  const className =
    variant === 'primary'
      ? 'k-btn k-btn--primary'
      : variant === 'ghost'
        ? 'k-btn k-btn--ghost'
        : 'k-btn';
  return (
    <button
      type={type}
      className={className}
      disabled={disabled}
      onClick={onClick}
      data-testid={testId}
      aria-label={ariaLabel}
    >
      {children}
    </button>
  );
}
