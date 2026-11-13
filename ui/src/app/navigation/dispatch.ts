import type { Direction, IntentEvent } from '@/shared/input/intents';
import { activateFocused, runBackHandler } from '@/shared/input/activation';
import { resolveIntent, type ResolvedAction } from './intentMap';
import type { ScreenEntry, ScreenKind } from './screens';
import { moveFocus } from './spatial';
import { useNavigation, selectTop } from './store';

export interface DispatchDeps {
  top: () => ScreenEntry;
  push: (kind: ScreenKind) => void;
  pop: () => boolean;
  moveFocus: (direction: Direction) => void;
  activate: () => boolean;
  runBackHandler: () => boolean;
  /** Acoes que dependem da tela (pular linhas, volume): ligadas nas telas das proximas tarefas. */
  onScreenAction?: (action: ResolvedAction, screen: ScreenKind) => void;
}

export const defaultDeps = (): DispatchDeps => ({
  top: () => selectTop(useNavigation.getState()),
  push: (kind) => useNavigation.getState().push(kind),
  pop: () => useNavigation.getState().pop(),
  moveFocus,
  activate: activateFocused,
  runBackHandler,
});

/** Intencao -> acao da camada do topo. Retorna a acao executada (ou null se ignorada). */
export function dispatchIntent(event: IntentEvent, deps: DispatchDeps): ResolvedAction | null {
  const screen = deps.top().kind;
  const action = resolveIntent(screen, event.intent);
  if (!action) return null;

  // Repeticao so serve para mover o foco (CA-07). A, B e atalhos nao repetem.
  if (event.repeat && action.type !== 'navigate') return null;

  switch (action.type) {
    case 'navigate':
      deps.moveFocus(action.direction);
      break;
    case 'activate':
      deps.activate();
      break;
    case 'back':
      if (!deps.runBackHandler()) deps.pop();
      break;
    case 'open':
      deps.push(action.screen);
      break;
    case 'page':
    case 'volume':
      deps.onScreenAction?.(action, screen);
      break;
  }
  return action;
}
