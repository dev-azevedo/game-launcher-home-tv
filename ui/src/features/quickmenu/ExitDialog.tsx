import type { ScreenEntry } from '@/app/navigation/screens';
import { useNavigation } from '@/app/navigation/store';
import { useLibrary } from '@/features/library/store';
import { openGameName } from '@/features/session/panel';
import { useSession } from '@/features/session/store';
import { ipc } from '@/shared/ipc';
import { ConfirmDialog } from '@/shared/ui/ConfirmDialog';

/** Sair do launcher (CA-05): o jogo em sessao continua aberto, e o aviso diz isso. */
export function ExitDialog({ active, entry }: { active: boolean; entry: ScreenEntry }) {
  const session = useSession((s) => s.session);
  const games = useLibrary((s) => s.games);
  const { remove } = useNavigation.getState();
  const openName = openGameName(session, games);
  return (
    <ConfirmDialog
      title="Sair do Sofá?"
      icon="exit"
      tone="danger"
      defaultFocus="cancel"
      destructive
      confirmLabel="Sair do Sofá"
      warning={openName ? `${openName} continuará aberto.` : undefined}
      warningTone="warn"
      active={active}
      onConfirm={() => void ipc.appExit().catch(() => remove(entry.key))}
      onCancel={() => remove(entry.key)}
    >
      O Sofá fecha e o Windows mostra a área de trabalho.
    </ConfirmDialog>
  );
}
