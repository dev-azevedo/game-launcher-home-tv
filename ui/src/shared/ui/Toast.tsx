import { useEffect } from 'react';
import { Icon } from './Icon';

interface ToastProps {
  message: string | null;
  onDone: () => void;
  durationMs?: number;
}

/** Confirmacao curta embaixo, sem foco (handoff 7). Some sozinha em 3,2 s. */
export function Toast({ message, onDone, durationMs = 3200 }: ToastProps) {
  useEffect(() => {
    if (!message) return;
    const timer = setTimeout(onDone, durationMs);
    return () => clearTimeout(timer);
  }, [message, onDone, durationMs]);
  if (!message) return null;
  return (
    <div className="toast-slot">
      <div className="toast" role="status" aria-live="polite">
        <Icon name="check" />
        <span>{message}</span>
      </div>
    </div>
  );
}
