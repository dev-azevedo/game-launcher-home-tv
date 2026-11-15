import type { ScreenEntry } from '@/app/navigation/screens';
import { useNavigation } from '@/app/navigation/store';
import { errorCopyFor } from '@/shared/domain/errors';
import { errorOf, ipc } from '@/shared/ipc';
import { ConfirmDialog } from '@/shared/ui/ConfirmDialog';
import { selectGameById, useLibrary } from './store';

interface Props {
  active: boolean;
  entry: ScreenEntry;
  /** Camada de opcoes por baixo: fecha junto ao confirmar. */
  parentKey?: string;
}

/** Ocultar com confirmacao (CA-21, RN-04, RN-14): "Cancelar" em foco; nunca apaga arquivo. */
export function HideConfirmDialog({ active, entry, parentKey }: Props) {
  const gameId = entry.params?.gameId;
  const name = useLibrary(selectGameById(gameId))?.name ?? 'o jogo';
  const { remove } = useNavigation.getState();

  const confirm = async () => {
    if (!gameId) return;
    try {
      await ipc.gameHide(gameId);
      useLibrary.getState().setNotice(`${name} oculto. Restaure em Configurações, Itens ocultos.`);
    } catch (thrown) {
      useLibrary.getState().setNotice(errorCopyFor(errorOf(thrown)).message);
    }
    remove(entry.key);
    if (parentKey) remove(parentKey);
  };

  return (
    <ConfirmDialog
      title={`Ocultar ${name}?`}
      icon="eyeoff"
      tone="danger"
      defaultFocus="cancel"
      destructive
      confirmLabel="Ocultar"
      active={active}
      onConfirm={() => void confirm()}
      onCancel={() => remove(entry.key)}
    >
      O jogo sai da biblioteca e não volta nas atualizações. Nenhum arquivo é apagado. Para restaurar, abra
      Configurações e escolha Itens ocultos.
    </ConfirmDialog>
  );
}
