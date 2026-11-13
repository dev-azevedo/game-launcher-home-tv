import { useEffect } from 'react';
import { useLatest } from '@/shared/useLatest';
import { getCurrentFocusKey } from '@noriginmedia/norigin-spatial-navigation';

/**
 * A UI trata A e B pela tabela de intencoes (o teclado do norigin fica desligado, ver spatial.ts).
 * Cada Focusable registra aqui o que A faz nele; a camada do topo pode registrar o que B faz.
 */
const activations = new Map<string, () => void>();

export function registerActivation(focusKey: string, fn: () => void): () => void {
  activations.set(focusKey, fn);
  return () => {
    if (activations.get(focusKey) === fn) activations.delete(focusKey);
  };
}

export function activateFocused(): boolean {
  const fn = activations.get(getCurrentFocusKey());
  if (!fn) return false;
  fn();
  return true;
}

const backHandlers: Array<() => void> = [];

/** Camada do topo substitui o "B volta" padrao (ex.: telas de espera, dialogos). */
export function useBackHandler(fn: (() => void) | undefined): void {
  const ref = useLatest(fn);
  const has = fn !== undefined;
  useEffect(() => {
    if (!has) return;
    const handler = () => ref.current?.();
    backHandlers.push(handler);
    return () => {
      const i = backHandlers.lastIndexOf(handler);
      if (i >= 0) backHandlers.splice(i, 1);
    };
  }, [has, ref]);
}

export function runBackHandler(): boolean {
  const handler = backHandlers[backHandlers.length - 1];
  if (!handler) return false;
  handler();
  return true;
}
