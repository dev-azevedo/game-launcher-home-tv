import { useEffect, useState } from 'react';
import type { ScreenEntry } from '@/app/navigation/screens';
import { useNavigation } from '@/app/navigation/store';
import { useLibrary } from '@/features/library/store';
import { openGameName } from '@/features/session/panel';
import { useSession } from '@/features/session/store';
import { ipc } from '@/shared/ipc';
import { ConfirmDialog } from '@/shared/ui/ConfirmDialog';
import { exitCopy } from './power';

/**
 * Fechar o launcher (CA-05, CA-82): so o SofaLauncher fecha, o jogo em sessao continua aberto e nada se perde.
 * "Cancelar" em foco; o botao de confirmar nao e vermelho (nao desliga o PC).
 */
export function ExitDialog({ active, entry }: { active: boolean; entry: ScreenEntry }) {
  const session = useSession((s) => s.session);
  const games = useLibrary((s) => s.games);
  const { remove } = useNavigation.getState();
  // Ate ler a configuracao vale o texto que nao promete reabrir ao reiniciar o PC (D-58).
  const [autostart, setAutostart] = useState(false);
  useEffect(() => {
    ipc.settingsGet().then((s) => setAutostart(s.autostart), () => undefined);
  }, []);
  const copy = exitCopy({ gameName: openGameName(session, games), autostart });
  return (
    <ConfirmDialog
      title={copy.title}
      ariaLabel="Fechar o launcher"
      icon="exit"
      defaultFocus="cancel"
      confirmKind="neutral"
      confirmIcon="exit"
      confirmLabel={copy.confirmLabel}
      warning={copy.warning}
      warningTone="warn"
      active={active}
      onConfirm={() => void ipc.appExit().catch(() => remove(entry.key))}
      onCancel={() => remove(entry.key)}
    >
      {copy.body.map((line) => (
        <p key={line}>{line}</p>
      ))}
    </ConfirmDialog>
  );
}
