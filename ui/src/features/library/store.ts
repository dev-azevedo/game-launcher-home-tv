import { create } from 'zustand';
import { events, ipc, type GameDto, type GameId, type LibrarySnapshot } from '@/shared/ipc';

export type LibraryStatus = 'loading' | 'ready' | 'error';

interface LibraryState {
  status: LibraryStatus;
  games: GameDto[];
  /** Jogo que deve receber o foco (jogo que fechou, CA-30), vindo do nucleo. */
  focusGameId: GameId | null;
  /** Chip "Atualizando biblioteca…" enquanto `library_refresh` roda. */
  refreshing: boolean;
  /** Mensagem curta de resultado ("Biblioteca atualizada."), some sozinha na tela. */
  notice: string | null;
  apply: (snapshot: LibrarySnapshot) => void;
  load: () => Promise<void>;
  refresh: () => Promise<void>;
  clearNotice: () => void;
  setNotice: (notice: string) => void;
}

export const useLibrary = create<LibraryState>((set, get) => ({
  status: 'loading',
  games: [],
  focusGameId: null,
  refreshing: false,
  notice: null,
  apply: (snapshot) =>
    set((s) => ({
      status: 'ready',
      games: snapshot.games,
      focusGameId: snapshot.focusGameId,
      // O resultado da varredura chega neste evento (docs/api #2).
      refreshing: false,
      notice: s.refreshing ? (snapshot.games.length ? 'Biblioteca atualizada.' : 'Nenhum jogo novo.') : s.notice,
    })),
  // Uma carga por invoke; depois so eventos (arch 3.4). Falha: a biblioteca fica vazia, sem tela de erro (CA-18).
  load: async () => {
    try {
      get().apply(await ipc.libraryGet());
    } catch {
      set({ status: 'error', games: [], focusGameId: null });
    }
  },
  refresh: async () => {
    if (get().refreshing) return;
    set({ refreshing: true, notice: null });
    try {
      await ipc.libraryRefresh();
    } catch {
      // `Busy` ou falha: sem varredura nova; a biblioteca atual segue valida.
      set({ refreshing: false });
    }
  },
  clearNotice: () => set({ notice: null }),
  setNotice: (notice) => set({ notice }),
}));

export const selectGameById = (id: GameId | null | undefined) => (s: LibraryState) =>
  id ? s.games.find((g) => g.id === id) : undefined;

/** Carga inicial + `library://changed`. Retorna o cancelamento. */
export function startLibrary(): () => void {
  const off = events.libraryChanged((snapshot) => useLibrary.getState().apply(snapshot));
  void useLibrary.getState().load();
  return off;
}
