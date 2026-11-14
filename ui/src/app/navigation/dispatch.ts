import type { Direction, IntentEvent } from '@/shared/input/intents';
import { activateFocused, runBackHandler } from '@/shared/input/activation';
import { resolveIntent, type ResolvedAction } from './intentMap';
import type { ScreenEntry, ScreenKind, ScreenParams } from './screens';
import { useFocusedGame } from './focusedGame';
import { runPageHandler } from './pageHandler';
import { moveFocus } from './spatial';
import { useNavigation, selectTop } from './store';

export interface DispatchDeps {
  top: () => ScreenEntry;
  push: (kind: ScreenKind, params?: ScreenParams) => void;
  /** Jogo sob o foco da biblioteca; View sem jogo em foco nao abre opcoes. */
  focusedGameId: () => string | null;
  pop: () => boolean;
  moveFocus: (direction: Direction) => void;
  activate: () => boolean;
  runBackHandler: () => boolean;
  /** Acoes que dependem da tela (pular linhas, volume): ligadas nas telas das proximas tarefas. */
  onScreenAction?: (action: ResolvedAction, screen: ScreenKind) => void;
}

export const defaultDeps = (): DispatchDeps => ({
  top: () => selectTop(useNavigation.getState()),
  push: (kind, params) => useNavigation.getState().push(kind, undefined, params),
  focusedGameId: () => useFocusedGame.getState().gameId,
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
      if (action.screen === 'gameOptions') {
        const gameId = deps.focusedGameId();
        if (!gameId) return null;
        deps.push('gameOptions', { gameId });
      } else {
        deps.push(action.screen);
      }
      break;
    case 'page':
      if (!runPageHandler(action.delta, action.rows)) deps.onScreenAction?.(action, screen);
      break;
    case 'volume':
      deps.onScreenAction?.(action, screen);
      break;
  }
  return action;
}
