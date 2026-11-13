import { useEffect } from 'react';
import { useLatest } from '@/shared/useLatest';
import type { IntentSource } from '@/shared/input/intents';
import { defaultDeps, dispatchIntent, type DispatchDeps } from './navigation/dispatch';

interface Props {
  /** Fonte unica: nucleo (`input://intent`) + teclado de emergencia, juntos por mergeSources. */
  source: IntentSource;
  deps?: DispatchDeps;
}

/** Liga a fonte de intencoes a camada de foco do topo (arch 3.2). Nao renderiza nada. */
export function InputBridge({ source, deps }: Props) {
  const depsRef = useLatest(deps);
  useEffect(
    () => source.subscribe((event) => dispatchIntent(event, depsRef.current ?? defaultDeps())),
    [source, depsRef],
  );
  return null;
}
