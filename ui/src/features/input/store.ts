import { create } from 'zustand';
import { events, ipc } from '@/shared/ipc';

export type ControllerBannerState = 'none' | 'warn' | 'lost' | 'ok';

/** "Controle conectado." fica 1,5 s e some (handoff 5; CA-15 pede em ate 2 s). */
export const CONTROLLER_OK_MS = 1500;

interface ControllerStore {
  /** Controles conectados; `null` ate a primeira leitura. */
  connected: number | null;
  banner: ControllerBannerState;
  apply: (connected: number) => void;
  load: () => Promise<void>;
}

let okTimer: ReturnType<typeof setTimeout> | undefined;

export const useControllers = create<ControllerStore>((set, get) => ({
  connected: null,
  banner: 'none',
  apply: (connected) => {
    const before = get().connected;
    clearTimeout(okTimer);
    if (connected === 0) {
      // Primeira leitura sem controle: aviso de abertura (CA-13). Depois de ter tido: desconexao (CA-14).
      set({ connected, banner: before === null ? 'warn' : 'lost' });
    } else if (before === 0) {
      // O aviso so vira "conectado" se havia aviso (CA-15).
      set({ connected, banner: 'ok' });
      okTimer = setTimeout(() => set({ banner: 'none' }), CONTROLLER_OK_MS);
    } else {
      set({ connected, banner: 'none' });
    }
  },
  load: async () => {
    try {
      get().apply((await ipc.inputStatus()).connected);
    } catch {
      // Sem leitura nao ha aviso: teclado e mouse seguem valendo (RN-07).
    }
  },
}));

/** Carga inicial + `input://controllers`. Retorna o cancelamento. */
export function startControllers(): () => void {
  const off = events.controllers((connected) => useControllers.getState().apply(connected));
  void useControllers.getState().load();
  return () => {
    off();
    clearTimeout(okTimer);
  };
}
