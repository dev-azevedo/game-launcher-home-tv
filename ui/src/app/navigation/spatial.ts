import { destroy, init, setKeyMap, navigateByDirection } from '@noriginmedia/norigin-spatial-navigation';
import type { Direction } from '@/shared/input/intents';

let started = false;

/** Codigo que nenhuma tecla gera: desliga a tecla no norigin (`setKeyMap` MESCLA com o padrao, nao substitui). */
const NO_KEY = [-1];

/**
 * Inicia a navegacao espacial. As teclas nativas do norigin ficam desligadas: toda entrada
 * (controle e teclado) chega como intencao pelo InputBridge, por uma rota so. Sem isso cada seta
 * moveria o foco duas vezes (uma pelo norigin, outra pela intencao).
 */
export function startSpatialNavigation(): void {
  if (started) return;
  started = true;
  init({ throttle: 0, shouldFocusDOMNode: false });
  setKeyMap({ left: NO_KEY, up: NO_KEY, right: NO_KEY, down: NO_KEY, enter: NO_KEY });
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
