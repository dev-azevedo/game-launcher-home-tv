import { create } from 'zustand';
import type { ScreenEntry, ScreenKind } from './screens';

interface NavigationState {
  stack: ScreenEntry[];
  push: (kind: ScreenKind, returnFocusKey?: string) => void;
  /** Desempilha uma camada. A raiz nunca sai (CA-10: a Home ignora B). */
  pop: () => boolean;
  /** Troca a camada do topo. */
  replace: (kind: ScreenKind) => void;
  /** Volta para a raiz. */
  reset: () => void;
}

let counter = 0;
const entry = (kind: ScreenKind, returnFocusKey?: string): ScreenEntry => ({
  key: `${kind}#${++counter}`,
  kind,
  returnFocusKey,
});

export const useNavigation = create<NavigationState>((set, get) => ({
  stack: [entry('home')],
  push: (kind, returnFocusKey) => set((s) => ({ stack: [...s.stack, entry(kind, returnFocusKey)] })),
  pop: () => {
    if (get().stack.length <= 1) return false;
    set((s) => ({ stack: s.stack.slice(0, -1) }));
    return true;
  },
  replace: (kind) => set((s) => ({ stack: [...s.stack.slice(0, -1), entry(kind)] })),
  reset: () => set((s) => ({ stack: s.stack.slice(0, 1) })),
}));

export const selectTop = (s: NavigationState): ScreenEntry => s.stack[s.stack.length - 1]!;
