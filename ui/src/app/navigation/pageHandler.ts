import { useEffect } from 'react';
import { useLatest } from '@/shared/useLatest';

type PageHandler = (delta: -1 | 1, rows: number) => void;

const handlers: PageHandler[] = [];

/** LB/RB: a tela ativa pula linhas (Home 1, Todos os jogos 3). So a camada do topo registra. */
export function usePageHandler(fn: PageHandler, active: boolean): void {
  const ref = useLatest(fn);
  useEffect(() => {
    if (!active) return;
    const handler: PageHandler = (delta, rows) => ref.current(delta, rows);
    handlers.push(handler);
    return () => {
      const i = handlers.lastIndexOf(handler);
      if (i >= 0) handlers.splice(i, 1);
    };
  }, [active, ref]);
}

export function runPageHandler(delta: -1 | 1, rows: number): boolean {
  const handler = handlers[handlers.length - 1];
  if (!handler) return false;
  handler(delta, rows);
  return true;
}
