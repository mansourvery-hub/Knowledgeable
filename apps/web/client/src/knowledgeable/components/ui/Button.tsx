import type { ReactNode } from 'react';

export interface ButtonProps {
  children: ReactNode;
  variant?: 'primary' | 'ghost' | 'default';
  type?: 'button' | 'submit';
  disabled?: boolean;
  title?: string;
  onClick?: () => void;
  testId?: string;
  ariaLabel?: string;
  className?: string;
}

export default function Button({
  children,
  variant = 'default',
  type = 'button',
  disabled,
  title,
  onClick,
  testId,
  ariaLabel,
  className: customClassName,
}: ButtonProps) {
  const variantClass =
    variant === 'primary'
      ? 'k-btn k-btn--primary'
      : variant === 'ghost'
        ? 'k-btn k-btn--ghost'
        : 'k-btn';
  const className = customClassName ? `${variantClass} ${customClassName}` : variantClass;
  return (
    <button
      type={type}
      className={className}
      disabled={disabled}
      title={title}
      onClick={onClick}
      data-testid={testId}
      aria-label={ariaLabel}
    >
      {children}
    </button>
  );
}
