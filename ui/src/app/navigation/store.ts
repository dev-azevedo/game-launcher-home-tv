import { create } from 'zustand';
import type { ScreenEntry, ScreenKind, ScreenParams } from './screens';

interface NavigationState {
  stack: ScreenEntry[];
  push: (kind: ScreenKind, returnFocusKey?: string, params?: ScreenParams) => void;
  /** Tira uma camada especifica (ex.: a de sessao, que some por evento do nucleo). */
  remove: (key: string) => void;
  /** Desempilha uma camada. A raiz nunca sai (CA-10: a Home ignora B). */
  pop: () => boolean;
  /** Troca a camada do topo. */
  replace: (kind: ScreenKind) => void;
  /** Volta para a raiz. */
  reset: () => void;
}

let counter = 0;
const entry = (kind: ScreenKind, returnFocusKey?: string, params?: ScreenParams): ScreenEntry => ({
  key: `${kind}#${++counter}`,
  kind,
  returnFocusKey,
  params,
});

export const useNavigation = create<NavigationState>((set, get) => ({
  stack: [entry('home')],
  push: (kind, returnFocusKey, params) =>
    set((s) => ({ stack: [...s.stack, entry(kind, returnFocusKey, params)] })),
  remove: (key) => set((s) => ({ stack: s.stack.filter((e, i) => i === 0 || e.key !== key) })),
  pop: () => {
    if (get().stack.length <= 1) return false;
    set((s) => ({ stack: s.stack.slice(0, -1) }));
    return true;
  },
  replace: (kind) => set((s) => ({ stack: [...s.stack.slice(0, -1), entry(kind)] })),
  reset: () => set((s) => ({ stack: s.stack.slice(0, 1) })),
}));

export const selectTop = (s: NavigationState): ScreenEntry => s.stack[s.stack.length - 1]!;
