import { useControllers } from '@/features/input/store';
import { useLibrary } from '@/features/library/store';
import { useSession } from '@/features/session/store';
import { useVolume } from '@/features/volume/store';
import { events, ipc } from '@/shared/ipc';

/**
 * Volta da suspensao (`system://resumed`, CA-46, RN-21): a tela atual NAO muda (a pilha de telas fica como
 * estava); so se recarrega o estado que pode ter mudado enquanto o PC dormia. Falha de leitura mantem o que
 * ja esta na tela (diferente da carga inicial, que mostra a biblioteca vazia).
 */
export function reloadAfterResume(): void {
  ipc.libraryGet().then((snapshot) => useLibrary.getState().apply(snapshot), () => undefined);
  void useSession.getState().load();
  void useVolume.getState().load();
  void useControllers.getState().load();
}

export function startSystemResume(): () => void {
  return events.systemResumed(reloadAfterResume);
}

/**
 * Segundo plano (RN-21): com a janela oculta ou minimizada o launcher nao deve gastar CPU/GPU que o jogo
 * precisa. Marca `data-window-hidden` no <html>; o CSS pausa toda animacao enquanto estiver oculta.
 */
export function startBackgroundGuard(doc: Document = document): () => void {
  const apply = () => {
    doc.documentElement.dataset.windowHidden = doc.visibilityState === 'hidden' ? 'true' : 'false';
  };
  apply();
  doc.addEventListener('visibilitychange', apply);
  return () => doc.removeEventListener('visibilitychange', apply);
}
