import { create } from 'zustand';
import { errorOf, events, ipc, type GameDto, type GameId, type SessionDto } from '@/shared/ipc';
import { IDLE_SESSION, type LaunchFailure } from './overlay';

interface SessionStore {
  session: SessionDto;
  pendingGameId: GameId | null;
  failure: LaunchFailure | null;
  replace: { currentGameId: GameId; newGameId: GameId } | null;

  setSession: (session: SessionDto) => void;
  load: () => Promise<void>;
  /** A num card (ou "Jogar" nas opcoes, CA-79): abre o jogo ou volta a ele se ja esta aberto (CA-36). */
  play: (game: GameDto) => Promise<void>;
  returnToGame: () => Promise<void>;
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

    setSession: (session) =>
      set((s) => ({
        session,
        // O nucleo ja respondeu: o "pedido sem resposta" acabou.
        pendingGameId: session.state === 'Idle' ? s.pendingGameId : null,
      })),

    load: async () => {
      try {
        get().setSession(await ipc.sessionGet());
      } catch {
        // Sem estado de sessao a biblioteca segue como Idle.
      }
    },

    play: async (game) => {
      if (get().pendingGameId || get().failure || get().replace) return;
      if (game.inProgress) return get().returnToGame();
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
      } catch (thrown) {
        fail({ error: errorOf(thrown), gameId: get().session.gameId ?? '' });
      }
    },

    confirmReplace: async () => {
      const replace = get().replace;
      if (!replace) return;
      set({ replace: null, pendingGameId: replace.newGameId });
      try {
        await ipc.sessionReplace(replace.newGameId);
      } catch (thrown) {
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
  const off = events.sessionState((session) => useSession.getState().setSession(session));
  void useSession.getState().load();
  return off;
}
