import { create } from 'zustand';
import { useNavigation } from '@/app/navigation/store';
import { useLibrary } from '@/features/library/store';
import {
  errorOf,
  events,
  ipc,
  type FrontOutcome,
  type GameDto,
  type GameId,
  type SessionDto,
  type SessionState,
} from '@/shared/ipc';
import { IDLE_SESSION, type LaunchFailure } from './overlay';

/** Estados que terminam com o jogo fechado: ao chegar em `Idle` vindo deles, avisa "<nome> fechado." */
const ENDING_STATES: readonly SessionState[] = ['Running', 'Ending', 'Closing', 'CloseOverdue'];

interface SessionStore {
  session: SessionDto;
  pendingGameId: GameId | null;
  failure: LaunchFailure | null;
  replace: { currentGameId: GameId; newGameId: GameId } | null;
  /** Voltou a "Fechando..." pelo card "Em andamento": vale ate o estado mudar (CA-78). */
  reopen: SessionState | null;
  /** Jogo que abre depois do fechamento da troca (CT-01). */
  swapTo: GameId | null;
  /** Como a combinacao de retorno trouxe o Sofa (`FrontAfterMinimize` mostra o aviso no painel). */
  panelOutcome: FrontOutcome | null;
  setSession: (session: SessionDto) => void;
  load: () => Promise<void>;
  /** A num card (ou "Jogar" nas opcoes, CA-79): abre o jogo ou volta a ele se ja esta aberto (CA-36). */
  play: (game: GameDto) => Promise<void>;
  /** `true` quando o nucleo trouxe o jogo a frente; erro vira dialogo (`ForegroundDenied`, `NoActiveSession`). */
  returnToGame: () => Promise<boolean>;
  closeGame: () => Promise<void>;
  forceClose: () => Promise<void>;
  /** Abre o painel do jogo em andamento (`session://panel`). */
  openPanel: (outcome: FrontOutcome) => void;
  /** Fecha o dialogo de erro e volta para a biblioteca (jogo segue aberto). */
  showLibrary: () => void;
  confirmReplace: () => Promise<void>;
  cancelReplace: () => void;
  closeFailure: () => void;
  /** "Voltar" e B nas telas de espera e nos avisos: nunca cancela o inicio (RN-31), docs/api #18. */
  dismiss: () => Promise<void>;
  /** "Continuar" do aviso de administrador: so aqui o UAC e pedido (CA-73, RN-27). */
  continueWithUac: () => Promise<void>;
  retry: () => Promise<void>;
  hideFailureGame: () => Promise<void>;
}

export const useSession = create<SessionStore>((set, get) => {
  const fail = (failure: LaunchFailure) => set({ failure, pendingGameId: null });

  return {
    session: IDLE_SESSION,
    pendingGameId: null,
    failure: null,
    replace: null,
    reopen: null,
    swapTo: null,
    panelOutcome: null,

    setSession: (session) => {
      const before = get().session;
      if (before.state !== session.state && session.state === 'Idle' && before.gameId && ENDING_STATES.includes(before.state)) {
        const name = useLibrary.getState().games.find((g) => g.id === before.gameId)?.name;
        if (name) useLibrary.getState().setNotice(`${name} fechado.`);
      }
      set((s) => {
        // Troca de jogo (CT-01): ao chegar a Idle o nucleo abre o novo; a espera ja aparece, sem piscar a biblioteca.
        const swap = session.state === 'Idle' ? s.swapTo : null;
        return {
          session,
          // O nucleo ja respondeu: o "pedido sem resposta" acabou.
          pendingGameId: session.state === 'Idle' ? (swap ?? s.pendingGameId) : null,
          reopen: s.session.state === session.state ? s.reopen : null,
          swapTo: ['Running', 'Closing', 'Ending'].includes(session.state) ? s.swapTo : null,
        };
      });
    },

    load: async () => {
      try {
        get().setSession(await ipc.sessionGet());
      } catch {
        // Sem estado de sessao a biblioteca segue como Idle.
      }
    },

    play: async (game) => {
      if (get().pendingGameId || get().failure || get().replace) return;
      if (game.inProgress) {
        // Fechando: nao ha "voltar ao jogo" (InvalidState). O card reabre a tela do estado: "Fechando..."
        // ou a oferta de forcar (CA-78, CA-35).
        const state = get().session.state;
        if (state === 'Closing' || state === 'CloseOverdue') set({ reopen: state });
        else await get().returnToGame();
        return;
      }
      set({ pendingGameId: game.id });
      try {
        await ipc.sessionLaunch(game.id);
      } catch (thrown) {
        const error = errorOf(thrown);
        if (error.code === 'SessionAlreadyActive' && error.params?.gameId) {
          set({ pendingGameId: null, replace: { currentGameId: error.params.gameId, newGameId: game.id } });
        } else if (error.code === 'NeedsElevation') {
          // O estado `NeedsElevation` chega por `session://state` e abre o aviso da 1a etapa.
          set({ pendingGameId: null });
        } else {
          fail({ error, gameId: game.id });
        }
      }
    },

    returnToGame: async () => {
      try {
        await ipc.sessionReturnToGame();
        // O jogo esta a frente e o Sofa em segundo plano: o painel nao deve ficar para a proxima vez.
        const nav = useNavigation.getState();
        nav.stack.filter((e) => e.kind === 'sessionPanel').forEach((e) => nav.remove(e.key));
        return true;
      } catch (thrown) {
        fail({ error: errorOf(thrown), gameId: get().session.gameId ?? '' });
        return false;
      }
    },
    closeGame: async () => {
      try {
        await ipc.sessionClose();
      } catch (thrown) {
        fail({ error: errorOf(thrown), gameId: get().session.gameId ?? '' });
      }
    },
    forceClose: async () => {
      try {
        await ipc.sessionForceClose();
      } catch (thrown) {
        // AccessDenied aqui e o jogo elevado (texto proprio); NotOverdue: ainda fechando.
        fail({ error: errorOf(thrown), gameId: get().session.gameId ?? '', command: 'session_force_close' });
      }
    },
    openPanel: (outcome) => {
      // `Failed`: o Sofa nao veio a frente; abrir a camada agora ficaria invisivel e presa.
      if (outcome === 'Failed') return;
      set({ panelOutcome: outcome });
      const nav = useNavigation.getState();
      if (!nav.stack.some((e) => e.kind === 'sessionPanel')) nav.push('sessionPanel');
    },
    showLibrary: () => {
      set({ failure: null });
      useNavigation.getState().reset();
    },

    confirmReplace: async () => {
      const replace = get().replace;
      if (!replace) return;
      set({ replace: null, pendingGameId: replace.newGameId, swapTo: replace.newGameId });
      try {
        await ipc.sessionReplace(replace.newGameId);
      } catch (thrown) {
        set({ swapTo: null });
        fail({ error: errorOf(thrown), gameId: replace.newGameId });
      }
    },

    cancelReplace: () => set({ replace: null }),
    closeFailure: () => set({ failure: null }),

    dismiss: async () => {
      // Otimista: a tela some ja; o evento do nucleo confirma `dismissed` (ou o novo estado).
      set((s) => ({
        pendingGameId: null,
        session:
          s.session.state === 'Launching' || s.session.state === 'Closing'
            ? { ...s.session, dismissed: true }
            : s.session,
      }));
      try {
        await ipc.sessionDismiss();
      } catch {
        // Idempotente no nucleo; falha aqui nao muda o que o usuario ve.
      }
    },

    continueWithUac: async () => {
      const gameId = get().session.gameId;
      if (!gameId) return;
      try {
        await ipc.sessionLaunchWithUac(gameId);
      } catch (thrown) {
        fail({ error: errorOf(thrown), gameId });
      }
    },

    retry: async () => {
      try {
        await ipc.sessionRetry();
      } catch (thrown) {
        fail({ error: errorOf(thrown), gameId: get().session.gameId ?? '' });
      }
    },

    hideFailureGame: async () => {
      const failure = get().failure;
      set({ failure: null });
      if (!failure) return;
      try {
        await ipc.gameHide(failure.error.params?.gameId ?? failure.gameId);
      } catch {
        // Jogo ja fora da biblioteca: nada a mostrar.
      }
    },
  };
});


/** Carga inicial + `session://state`. Retorna o cancelamento. */
export function startSession(): () => void {
  const offState = events.sessionState((session) => useSession.getState().setSession(session));
  const offPanel = events.sessionPanel((outcome) => useSession.getState().openPanel(outcome));
  void useSession.getState().load();
  return () => {
    offState();
    offPanel();
  };
}
