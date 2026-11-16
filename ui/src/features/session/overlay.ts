import type { DialogCover } from '@/shared/ui/DialogSide';
import type { IconName } from '@/shared/ui/Icon';
import type { ErrorDto, GameDto, GameId, SessionDto, SessionState } from '@/shared/ipc';
import { findGame } from '@/features/library/domain';
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
  | { kind: 'closing'; gameId: GameId; swapTo: GameId | null }
  | { kind: 'overdue'; gameId: GameId }
  | { kind: 'failure'; failure: LaunchFailure }
  | { kind: 'replace'; currentGameId: GameId; newGameId: GameId };

export interface OverlayInput {
  session: SessionDto;
  /** Jogo pedido e ainda sem resposta do nucleo (entre o A e o primeiro evento). */
  pendingGameId: GameId | null;
  failure: LaunchFailure | null;
  replace: { currentGameId: GameId; newGameId: GameId } | null;
  /** Estado em que o usuario voltou a "Fechando..." pelo card "Em andamento" (cancela o `dismissed` local). */
  reopen?: SessionState | null;
  /** Jogo que abre depois do fechamento (troca de jogo, CT-01): mostrado em "Fechando...". */
  swapTo?: GameId | null;
}

/**
 * Regra pura: estado do nucleo + resultado do ultimo comando -> tela de sessao.
 * `dismissed` (Voltar ou B na espera) evita reabrir a tela a cada evento (docs/api D9).
 */
export function launchOverlay({ session, pendingGameId, failure, replace, reopen = null, swapTo = null }: OverlayInput): LaunchOverlay | null {
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
    case 'Closing':
      // CA-78: "Ir para a biblioteca" dispensa a espera; o card "Em andamento" a reabre (`reopen`).
      if (!gameId || (session.dismissed && reopen !== 'Closing')) return null;
      return { kind: 'closing', gameId, swapTo };
    case 'CloseOverdue':
      // Segunda confirmacao (RN-12): decisao do usuario, nunca fica escondida por `dismissed`.
      return gameId ? { kind: 'overdue', gameId } : null;
    default:
      // Running e Ending: o painel e aberto por `session://panel`, nao por estado.
      return null;
  }
}

export type DialogAction =
  | 'back'
  | 'hide'
  | 'retry'
  | 'continue'
  | 'replace'
  | 'library'
  | 'cancel'
  | 'keepRunning'
  | 'force';

export interface DialogSpec {
  /** Nome do jogo (ou frase curta da spec); a mensagem vai em `body` (DES-BUG-01). */
  title: string;
  body?: string;
  tone: 'warn' | 'danger' | 'ok';
  icon?: IconName;
  /** Capa do jogo no lado esquerdo; sem ela, o icone tematico. */
  cover?: DialogCover | null;
  /** Esquerda: B tambem aciona este botao. */
  cancel: { label: string; action: DialogAction };
  /** Direita, quando existe. `kind` neutro = acao secundaria (so o primario e ambar, DES-BUG-02). */
  confirm?: { label: string; action: DialogAction; destructive?: boolean; kind?: 'primary' | 'neutral'; icon?: IconName };
  /** Aviso em destaque abaixo do texto (ex.: progresso nao salvo sera perdido). */
  warning?: string;
  defaultFocus: 'cancel' | 'confirm';
}

/** Tela de espera (RN-31): titulo, texto, rotulo do spinner e o unico botao. */
export interface WaitSpec {
  title: string;
  body: string;
  spinLabel: string;
  buttonLabel: string;
  /** `dismiss`: so interrompe a espera; `library`: tambem volta a biblioteca (CA-78). */
  leave: 'dismiss' | 'library';
  cover?: DialogCover | null;
  icon?: IconName;
}

const nameOf = (games: readonly GameDto[], id: GameId | undefined): string | undefined => findGame(games, id)?.name;

function coverOf(games: readonly GameDto[], id: GameId | undefined): DialogCover | null {
  const game = findGame(games, id);
  return game ? { id: game.id, name: game.name, hasCover: game.cover !== '' } : null;
}

/** Tela de espera de cada estado (abrindo, esperando o UAC, fechando); `null` nos demais. */
export function overlayWait(overlay: LaunchOverlay, games: readonly GameDto[]): WaitSpec | null {
  switch (overlay.kind) {
    case 'launching':
      return {
        title: `Abrindo ${nameOf(games, overlay.gameId) ?? 'o jogo'}…`,
        body: 'Aguarde. O jogo vai aparecer na frente.',
        spinLabel: 'Iniciando',
        buttonLabel: 'Voltar',
        leave: 'dismiss',
        cover: coverOf(games, overlay.gameId),
        icon: 'play',
      };
    case 'uacWait': {
      const name = nameOf(games, overlay.gameId) ?? 'o jogo';
      return {
        title: name,
        body: `${name} pede permissão de administrador. Confirme no Windows com teclado ou mouse. A combinação de retorno pode não funcionar neste jogo.`,
        spinLabel: 'Esperando o Windows',
        buttonLabel: 'Voltar',
        leave: 'dismiss',
        icon: 'shield',
      };
    }
    case 'closing': {
      const swap = overlay.swapTo ? (nameOf(games, overlay.swapTo) ?? 'o jogo') : null;
      return {
        title: `Fechando ${nameOf(games, overlay.gameId) ?? 'o jogo'}…`,
        body: swap ? `Esperando o jogo fechar. Depois o SofaLauncher abre ${swap}.` : 'Esperando o jogo fechar. Pode levar alguns segundos.',
        spinLabel: 'Fechando',
        buttonLabel: 'Ir para a biblioteca',
        leave: 'library',
        cover: coverOf(games, overlay.gameId),
        icon: 'power',
      };
    }
    default:
      return null;
  }
}

/** Textos do contrato (microcopy secao 2) para cada estado que vira dialogo. Esperas: `overlayWait`. */
export function overlayDialog(overlay: LaunchOverlay, games: readonly GameDto[]): DialogSpec | null {
  switch (overlay.kind) {
    case 'launching':
    case 'uacWait':
    case 'closing':
      return null;
    case 'overdue':
      return {
        title: `${nameOf(games, overlay.gameId) ?? 'O jogo'} não fechou`,
        body: 'O jogo não respondeu ao pedido para fechar.',
        warning: 'O progresso não salvo será perdido.',
        tone: 'danger',
        cover: coverOf(games, overlay.gameId),
        cancel: { label: 'Cancelar', action: 'keepRunning' },
        confirm: { label: 'Forçar fechamento', action: 'force', destructive: true },
        defaultFocus: 'cancel',
      };
    case 'needsElevation': {
      const name = nameOf(games, overlay.gameId);
      const copy = errorCopyFor({ code: 'NeedsElevation' }, { gameName: name });
      return {
        title: name ?? 'Jogo',
        body: copy.message,
        tone: 'warn',
        icon: 'shield',
        cancel: { label: 'Voltar', action: 'back' },
        confirm: { label: 'Continuar', action: 'continue' },
        defaultFocus: 'confirm',
      };
    }
    case 'elevationDenied': {
      const name = nameOf(games, overlay.gameId);
      const copy = errorCopyFor({ code: 'ElevationDenied' }, { gameName: name });
      return {
        title: name ?? 'Jogo',
        body: copy.message,
        tone: 'danger',
        icon: 'shield',
        cover: coverOf(games, overlay.gameId),
        cancel: { label: 'Voltar', action: 'back' },
        defaultFocus: 'cancel',
      };
    }
    case 'notConfirmed': {
      const name = nameOf(games, overlay.gameId);
      return {
        title: name ?? 'Jogo',
        body: `Não foi possível confirmar que ${name ?? 'o jogo'} abriu.`,
        tone: 'warn',
        cover: coverOf(games, overlay.gameId),
        cancel: { label: 'Voltar', action: 'back' },
        confirm: { label: 'Tentar de novo', action: 'retry', icon: 'refresh' },
        defaultFocus: 'confirm',
      };
    }
    case 'replace': {
      const current = nameOf(games, overlay.currentGameId) ?? 'Outro jogo';
      const next = nameOf(games, overlay.newGameId) ?? 'o jogo';
      return {
        // A frase da spec (CA-31) fica como titulo.
        title: `${current} está aberto. Fechar e abrir ${next}?`,
        body: `O progresso não salvo de ${current} pode ser perdido.`,
        tone: 'danger',
        cover: coverOf(games, overlay.currentGameId),
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
  const id = error.params?.gameId ?? gameId;
  const name = nameOf(games, id);
  const copy = errorCopyFor(error, { gameName: name, command });
  const cover = coverOf(games, id);
  const back = { label: 'Voltar', action: 'back' as const };
  switch (error.code) {
    // CA-28 e CA-71: Voltar e Ocultar, foco em Voltar. "Ocultar" e acao secundaria (neutra).
    case 'GameFileMissing':
    case 'NetworkPathBlocked':
      return {
        title: name ?? 'Jogo',
        body: copy.message,
        tone: 'warn',
        cover,
        cancel: back,
        confirm: { label: 'Ocultar', action: 'hide', kind: 'neutral', icon: 'eyeoff' },
        defaultFocus: 'cancel',
      };
    case 'ForegroundDenied':
      return {
        title: name ?? 'Jogo',
        body: copy.message,
        tone: 'warn',
        cover,
        cancel: { label: 'Ir para a biblioteca', action: 'library' },
        confirm: { label: 'Tentar de novo', action: 'retry', icon: 'refresh' },
        defaultFocus: 'confirm',
      };
    case 'StoreNotInstalled':
      return { title: name ?? 'Jogo', body: copy.message, tone: 'warn', cover, cancel: back, defaultFocus: 'cancel' };
    case 'AccessDenied':
      // Forcar fechamento recusado (jogo elevado): o titulo e o jogo.
      if (command === 'session_force_close') {
        return { title: name ?? 'Jogo', body: copy.message, tone: 'danger', icon: 'shield', cover, cancel: back, defaultFocus: 'cancel' };
      }
      return { title: copy.message, body: copy.support, tone: 'warn', cancel: back, defaultFocus: 'cancel' };
    default:
      return { title: copy.message, body: copy.support, tone: 'warn', cancel: back, defaultFocus: 'cancel' };
  }
}
