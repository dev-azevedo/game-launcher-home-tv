import { destroy, init, setKeyMap, navigateByDirection } from '@noriginmedia/norigin-spatial-navigation';
import type { Direction } from '@/shared/input/intents';

let started = false;

/**
 * Inicia a navegacao espacial. O mapa de teclas do norigin fica vazio: toda entrada
 * (controle e teclado) chega como intencao pelo InputBridge, por uma rota so.
 */
export function startSpatialNavigation(): void {
  if (started) return;
  started = true;
  init({ throttle: 0, shouldFocusDOMNode: false });
  setKeyMap({});
}

export function moveFocus(direction: Direction): void {
  void navigateByDirection(direction);
}

/** Reinicia a arvore de foco (testes: o estado do norigin e global entre casos). */
export function resetSpatialNavigation(): void {
  destroy();
  started = false;
  startSpatialNavigation();
}
