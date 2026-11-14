import { useEffect } from 'react';
import type { Direction } from '@/shared/input/intents';
import { useLatest } from '@/shared/useLatest';

/** Devolve `true` quando consumiu a direcao (o foco nao se move). */
type DirectionHandler = (direction: Direction) => boolean;

const handlers: DirectionHandler[] = [];

/**
 * Direcao tratada pela propria tela antes de mover o foco: ←/→ no Volume do menu rapido mudam o nivel,
 * ←/→ nas configuracoes trocam de zona. So a camada do topo registra.
 */
export function useDirectionHandler(fn: DirectionHandler, active: boolean): void {
  const ref = useLatest(fn);
  useEffect(() => {
    if (!active) return;
    const handler: DirectionHandler = (direction) => ref.current(direction);
    handlers.push(handler);
    return () => {
      const i = handlers.lastIndexOf(handler);
      if (i >= 0) handlers.splice(i, 1);
    };
  }, [active, ref]);
}

export function runDirectionHandler(direction: Direction): boolean {
  const handler = handlers[handlers.length - 1];
  return handler ? handler(direction) : false;
}
