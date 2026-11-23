import { create } from 'zustand';
import { cycleLayout, parseLayout, type LayoutId } from './domain';

/** Chave no armazenamento do WebView (preferencia so visual, por usuario do Windows). */
export const LAYOUT_STORAGE_KEY = 'sofa.layout';

function readSaved(): LayoutId {
  try {
    return parseLayout(window.localStorage.getItem(LAYOUT_STORAGE_KEY));
  } catch {
    // Armazenamento bloqueado ou indisponivel: fica o padrao.
    return parseLayout(null);
  }
}

function save(id: LayoutId): void {
  try {
    window.localStorage.setItem(LAYOUT_STORAGE_KEY, id);
  } catch {
    // Sem armazenamento a escolha vale so ate fechar o app.
  }
}

/** O CSS troca cores e medidas por `html[data-layout]`; a Home troca a estrutura pela store. */
function apply(id: LayoutId): void {
  document.documentElement.dataset.layout = id;
}

interface AppearanceStore {
  layout: LayoutId;
  setLayout: (id: LayoutId) => void;
  cycle: (delta: 1 | -1) => void;
}

export const useAppearance = create<AppearanceStore>((set, get) => ({
  layout: readSaved(),
  setLayout: (id) => {
    if (get().layout === id) return;
    save(id);
    apply(id);
    set({ layout: id });
  },
  cycle: (delta) => get().setLayout(cycleLayout(get().layout, delta)),
}));

/** Aplica o layout salvo antes do primeiro quadro (main.tsx). */
export function startAppearance(): void {
  const id = readSaved();
  apply(id);
  useAppearance.setState({ layout: id });
}
