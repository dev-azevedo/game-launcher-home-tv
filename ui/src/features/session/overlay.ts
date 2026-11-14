import type { ErrorDto, GameDto, GameId, SessionDto } from '@/shared/ipc';
import { errorCopyFor } from '@/shared/domain/errors';

/** Erro de um comando de inicio (`session_launch`, `session_replace`...) mostrado em dialogo. */
export interface LaunchFailure {
  error: ErrorDto;
  /** Jogo que o usuario tentou abrir (o `params.gameId` pode ser o jogo ja aberto). */
  gameId: GameId;
  command?: 'session_force_close';
}

export const IDLE_SESSION: SessionDto = {
  state: 'Idle',
  gameId: null,
  since: null,
  dismissed: false,
  elevated: false,
  minimized: false,
  launchFailure: null,
};

/** O que a camada de sessao mostra agora (FE-08). `null` = nada: a biblioteca aparece. */
export type LaunchOverlay =
  | { kind: 'launching'; gameId: GameId }
  | { kind: 'uacWait'; gameId: GameId }
  | { kind: 'needsElevation'; gameId: GameId }
  | { kind: 'elevationDenied'; gameId: GameId }
  | { kind: 'notConfirmed'; gameId: GameId }
  | { kind: 'failure'; failure: LaunchFailure }
  | { kind: 'replace'; currentGameId: GameId; newGameId: GameId };

export interface OverlayInput {
  session: SessionDto;
  /** Jogo pedido e ainda sem resposta do nucleo (entre o A e o primeiro evento). */
  pendingGameId: GameId | null;
  failure: LaunchFailure | null;
  replace: { currentGameId: GameId; newGameId: GameId } | null;
}

/**
 * Regra pura: estado do nucleo + resultado do ultimo comando -> tela de sessao.
 * `dismissed` (Voltar ou B na espera) evita reabrir a tela a cada evento (docs/api D9).
 */
export function launchOverlay({ session, pendingGameId, failure, replace }: OverlayInput): LaunchOverlay | null {
  if (replace) return { kind: 'replace', ...replace };
  if (failure) return { kind: 'failure', failure };
  const gameId = session.gameId;
  switch (session.state) {
    case 'Idle':
      return pendingGameId ? { kind: 'launching', gameId: pendingGameId } : null;
    case 'Launching':
      if (!gameId || session.dismissed) return null;
      return session.elevated ? { kind: 'uacWait', gameId } : { kind: 'launching', gameId };
    case 'NeedsElevation':
      if (!gameId) return null;
      // UAC recusado volta a este estado com `launchFailure` (CA-73).
      return session.launchFailure === 'ElevationDenied'
        ? { kind: 'elevationDenied', gameId }
        : { kind: 'needsElevation', gameId };
    case 'NotConfirmed':
      return gameId ? { kind: 'notConfirmed', gameId } : null;
    default:
      // Running, Ending, Closing e CloseOverdue: painel e "Fechando" sao do FE-09.
      return null;
  }
}

export type DialogAction = 'back' | 'hide' | 'retry' | 'continue' | 'replace' | 'library' | 'cancel';

export interface DialogSpec {
  title: string;
  body?: string;
  tone: 'warn' | 'danger' | 'ok';
  /** Esquerda: B tambem aciona este botao. */
  cancel: { label: string; action: DialogAction };
  /** Direita, quando existe. */
  confirm?: { label: string; action: DialogAction; destructive?: boolean };
  defaultFocus: 'cancel' | 'confirm';
}

const nameOf = (games: readonly GameDto[], id: GameId | undefined): string | undefined =>
  id ? games.find((g) => g.id === id)?.name : undefined;

/** Textos do contrato (microcopy secao 2) para cada estado que vira dialogo. Esperas nao entram aqui. */
export function overlayDialog(overlay: LaunchOverlay, games: readonly GameDto[]): DialogSpec | null {
  switch (overlay.kind) {
    case 'launching':
    case 'uacWait':
      return null;
    case 'needsElevation': {
      const copy = errorCopyFor({ code: 'NeedsElevation' }, { gameName: nameOf(games, overlay.gameId) });
      return {
        title: copy.message,
        tone: 'warn',
        cancel: { label: 'Voltar', action: 'back' },
        confirm: { label: 'Continuar', action: 'continue' },
        defaultFocus: 'confirm',
      };
    }
    case 'elevationDenied': {
      const copy = errorCopyFor({ code: 'ElevationDenied' }, { gameName: nameOf(games, overlay.gameId) });
      return { title: copy.message, tone: 'danger', cancel: { label: 'Voltar', action: 'back' }, defaultFocus: 'cancel' };
    }
    case 'notConfirmed': {
      const name = nameOf(games, overlay.gameId) ?? 'o jogo';
      return {
        title: `Não foi possível confirmar que ${name} abriu.`,
        tone: 'warn',
        cancel: { label: 'Voltar', action: 'back' },
        confirm: { label: 'Tentar de novo', action: 'retry' },
        defaultFocus: 'confirm',
      };
    }
    case 'replace': {
      const current = nameOf(games, overlay.currentGameId) ?? 'Outro jogo';
      const next = nameOf(games, overlay.newGameId) ?? 'o jogo';
      return {
        title: `${current} está aberto. Fechar e abrir ${next}?`,
        body: `O progresso não salvo de ${current} pode ser perdido.`,
        tone: 'danger',
        cancel: { label: 'Cancelar', action: 'cancel' },
        confirm: { label: 'Fechar e abrir', action: 'replace', destructive: true },
        defaultFocus: 'cancel',
      };
    }
    case 'failure':
      return failureDialog(overlay.failure, games);
  }
}

function failureDialog({ error, gameId, command }: LaunchFailure, games: readonly GameDto[]): DialogSpec {
  const copy = errorCopyFor(error, { gameName: nameOf(games, error.params?.gameId ?? gameId), command });
  switch (error.code) {
    // CA-28 e CA-71: Ocultar e Voltar, foco em Voltar.
    case 'GameFileMissing':
    case 'NetworkPathBlocked':
      return {
        title: copy.message,
        body: copy.support,
        tone: 'warn',
        cancel: { label: 'Voltar', action: 'back' },
        confirm: { label: 'Ocultar', action: 'hide' },
        defaultFocus: 'cancel',
      };
    case 'ForegroundDenied':
      return {
        title: copy.message,
        tone: 'warn',
        cancel: { label: 'Ir para a biblioteca', action: 'library' },
        confirm: { label: 'Tentar de novo', action: 'retry' },
        defaultFocus: 'confirm',
      };
    default:
      return { title: copy.message, body: copy.support, tone: 'warn', cancel: { label: 'Voltar', action: 'back' }, defaultFocus: 'cancel' };
  }
}
