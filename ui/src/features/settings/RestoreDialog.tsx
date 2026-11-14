import type { ScreenEntry } from '@/app/navigation/screens';
import { useNavigation } from '@/app/navigation/store';
import { ConfirmDialog } from '@/shared/ui/ConfirmDialog';
import { useSettings } from './store';

/** Restaurar item oculto (CA-70): "Restaurar" em foco, nao e destrutivo. Nome e data de ultimo jogo voltam. */
export default function RestoreDialog({ active, entry }: { active: boolean; entry: ScreenEntry }) {
  const gameId = entry.params?.gameId;
  const name = useSettings((s) => s.hidden.find((g) => g.id === gameId)?.name ?? 'o jogo');
  const { remove } = useNavigation.getState();
  return (
    <ConfirmDialog
      title={`Restaurar ${name}?`}
      icon="refresh"
      tone="ok"
      defaultFocus="confirm"
      confirmLabel="Restaurar"
      active={active}
      onConfirm={() => {
        remove(entry.key);
        if (gameId) void useSettings.getState().restore(gameId);
      }}
      onCancel={() => remove(entry.key)}
    >
      O jogo volta para a biblioteca com o nome e a data de último jogo que tinha.
    </ConfirmDialog>
  );
}
