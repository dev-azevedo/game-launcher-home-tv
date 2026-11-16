import type { ScreenEntry } from '@/app/navigation/screens';
import { useNavigation } from '@/app/navigation/store';
import { selectGameById, useLibrary } from '@/features/library/store';
import { ConfirmDialog } from '@/shared/ui/ConfirmDialog';
import { useSession } from './store';

/** Fechar jogo (CA-35, RN-12, 1a confirmacao): "Cancelar" em foco; so pede o fechamento normal. */
export function CloseConfirmDialog({ active, entry }: { active: boolean; entry: ScreenEntry }) {
  const gameId = entry.params?.gameId;
  const name = useLibrary(selectGameById(gameId))?.name ?? 'o jogo';
  const closeGame = useSession((s) => s.closeGame);
  const { remove } = useNavigation.getState();
  return (
    <ConfirmDialog
      title={`Fechar ${name}?`}
      icon="power"
      tone="danger"
      defaultFocus="cancel"
      destructive
      confirmLabel="Fechar jogo"
      warning="O progresso não salvo pode ser perdido."
      active={active}
      onConfirm={() => {
        remove(entry.key);
        // Fecha a confirmacao e pede o fechamento; "Fechando..." chega por `session://state` (Closing).
        void closeGame();
      }}
      onCancel={() => remove(entry.key)}
    >
      O SofaLauncher pede ao jogo para fechar.
    </ConfirmDialog>
  );
}
