import { create } from 'zustand';

/** Jogo sob o foco da biblioteca (Home ou Todos os jogos). View abre as opcoes dele (CA-79). */
export const useFocusedGame = create<{ gameId: string | null; set: (gameId: string | null) => void }>((set) => ({
  gameId: null,
  set: (gameId) => set({ gameId }),
}));
