import { create } from 'zustand';
import type { IntentSource } from './intents';

/** Por onde o usuario esta mexendo no launcher: muda as dicas de botao (controle ou teclado). */
export type InputMode = 'gamepad' | 'keyboard';

interface InputModeStore {
  mode: InputMode;
  setMode: (mode: InputMode) => void;
}

/** Comeca no controle (produto de 10 pes); a ultima entrada usada vale. */
export const useInputMode = create<InputModeStore>((set, get) => ({
  mode: 'gamepad',
  setMode: (mode) => {
    if (get().mode !== mode) set({ mode });
  },
}));

/** Marca o modo de entrada a cada intencao que a fonte entrega, antes de repassar. */
export function withInputMode(source: IntentSource, mode: InputMode): IntentSource {
  return {
    subscribe(handler) {
      return source.subscribe((event) => {
        useInputMode.getState().setMode(mode);
        handler(event);
      });
    },
  };
}
