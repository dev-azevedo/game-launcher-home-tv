import { create } from 'zustand';
import { useNavigation } from '@/app/navigation/store';
import { useLibrary } from '@/features/library/store';
import { useSession } from '@/features/session/store';
import { errorCopyFor } from '@/shared/domain/errors';
import { errorOf, events, ipc, type UpdateDto, type UpdateProgressEvent } from '@/shared/ipc';
import { STARTUP_CHECK_DELAY_MS } from './domain';

export type UpdatePhase = 'idle' | 'checking' | 'downloading';

interface UpdateStore {
  /** Versao nova encontrada na ultima procura. */
  available: UpdateDto | null;
  phase: UpdatePhase;
  progress: UpdateProgressEvent | null;
  /** O aviso automatico ja apareceu nesta abertura (nao insiste a cada volta a tela inicial). */
  offered: boolean;
  /** Procura versao nova. `manual` (Configuracoes) avisa tambem "ja esta na mais nova" e falha. */
  check: (manual: boolean) => Promise<void>;
  /** Baixa e abre o instalador. So volta em erro (em sucesso o launcher fecha). */
  install: () => Promise<void>;
  setProgress: (progress: UpdateProgressEvent) => void;
}

const notice = (text: string) => useLibrary.getState().setNotice(text);

/** Abre o dialogo de atualizacao por cima do que estiver na tela. */
export function openUpdateDialog(): void {
  const { stack, push } = useNavigation.getState();
  if (stack.some((e) => e.kind === 'confirmDialog' && e.params?.confirm === 'update')) return;
  push('confirmDialog', undefined, { confirm: 'update' });
}

export const useUpdate = create<UpdateStore>((set, get) => ({
  available: null,
  phase: 'idle',
  progress: null,
  offered: false,
  check: async (manual) => {
    if (get().phase !== 'idle') return;
    set({ phase: 'checking' });
    try {
      const available = await ipc.updateCheck();
      set({ available, phase: 'idle' });
      if (available) {
        if (manual) {
          set({ offered: true });
          openUpdateDialog();
        }
      } else if (manual) {
        notice('Você já está na versão mais nova.');
      }
    } catch (thrown) {
      set({ phase: 'idle' });
      // Procura automatica sem internet nao incomoda: so a manual mostra o erro.
      if (manual) notice(errorCopyFor(errorOf(thrown)).message);
    }
  },
  install: async () => {
    if (get().phase !== 'idle') return;
    set({ phase: 'downloading', progress: null });
    try {
      await ipc.updateInstall();
    } catch (thrown) {
      const dto = errorOf(thrown);
      notice(
        dto.code === 'InvalidState'
          ? 'Feche o jogo antes de atualizar o SofaLauncher.'
          : errorCopyFor(dto).message,
      );
    } finally {
      set({ phase: 'idle', progress: null });
    }
  },
  setProgress: (progress) => set({ progress }),
}));

/** Mostra o aviso automatico quando ha versao nova e nenhum jogo em sessao. */
function offerIfReady(): void {
  const { available, offered, phase } = useUpdate.getState();
  if (!available || offered || phase !== 'idle') return;
  if (useSession.getState().session.state !== 'Idle') return;
  useUpdate.setState({ offered: true });
  openUpdateDialog();
}

/**
 * Ao abrir o launcher: procura versao nova em segundo plano (depois de `STARTUP_CHECK_DELAY_MS`) e,
 * se houver, pergunta se quer atualizar. Com jogo aberto, espera a sessao voltar a `Idle`.
 */
export function startUpdates(delayMs: number = STARTUP_CHECK_DELAY_MS): () => void {
  const offProgress = events.updateProgress((p) => useUpdate.getState().setProgress(p));
  const timer = setTimeout(() => {
    void useUpdate
      .getState()
      .check(false)
      .then(offerIfReady);
  }, delayMs);
  const offSession = useSession.subscribe((s, prev) => {
    if (s.session.state === 'Idle' && prev.session.state !== 'Idle') offerIfReady();
  });
  return () => {
    clearTimeout(timer);
    offProgress();
    offSession();
  };
}
