import { useEffect, useRef, useState } from 'react';
import type { ScreenEntry } from '@/app/navigation/screens';
import { useNavigation } from '@/app/navigation/store';
import { useLibrary } from '@/features/library/store';
import { openGameName } from '@/features/session/panel';
import { useSession } from '@/features/session/store';
import { errorCopyFor } from '@/shared/domain/errors';
import { errorOf, events, ipc, type ErrorDto } from '@/shared/ipc';
import { ConfirmDialog } from '@/shared/ui/ConfirmDialog';
import { WaitScreen } from '@/shared/ui/WaitScreen';
import { openGameWarning, powerCopy } from './power';

type Phase = 'confirm' | 'running' | { error: ErrorDto };

/**
 * Confirmacao de desligar, reiniciar ou suspender (CA-43 a CA-48, SEC-27). `power_prepare` roda ao
 * abrir (ticket de 30 s, uso unico); `power_execute` so ao confirmar. "Cancelar" em foco; B cancela.
 */
export function PowerDialog({ active, entry }: { active: boolean; entry: ScreenEntry }) {
  const action = entry.params?.power ?? 'Shutdown';
  const session = useSession((s) => s.session);
  const games = useLibrary((s) => s.games);
  const { remove } = useNavigation.getState();
  const [phase, setPhase] = useState<Phase>('confirm');
  const [autostart, setAutostart] = useState(false);
  const ticket = useRef<Promise<string> | null>(null);
  const busy = useRef(false);

  useEffect(() => {
    const prepared = ipc.powerPrepare(action);
    ticket.current = prepared;
    prepared.catch((thrown) => setPhase({ error: errorOf(thrown) }));
    if (action === 'Restart') ipc.settingsGet().then((s) => setAutostart(s.autostart), () => undefined);
  }, [action]);

  // Ao acordar da suspensao o Sofa volta para a tela em que estava (CA-46).
  useEffect(() => {
    if (phase !== 'running' || action !== 'Suspend') return;
    return events.systemResumed(() => remove(entry.key));
  }, [phase, action, remove, entry.key]);

  const copy = powerCopy(action, { autostart });
  const close = () => remove(entry.key);

  const confirm = async () => {
    if (busy.current || !ticket.current) return;
    busy.current = true;
    try {
      await ipc.powerExecute(await ticket.current);
      setPhase('running');
    } catch (thrown) {
      setPhase({ error: errorOf(thrown) });
    } finally {
      busy.current = false;
    }
  };

  if (phase === 'running') {
    return (
      <WaitScreen title={copy.runningTitle} buttonLabel="Voltar" onLeave={close}>
        {copy.runningBody}
      </WaitScreen>
    );
  }
  if (phase !== 'confirm') {
    return (
      <ConfirmDialog
        key="error"
        title={errorCopyFor(phase.error).message}
        icon="alert"
        defaultFocus="cancel"
        cancelLabel="Voltar"
        active={active}
        onCancel={close}
      />
    );
  }
  const openName = openGameName(session, games);
  return (
    <ConfirmDialog
      key="confirm"
      title={copy.title}
      icon={copy.icon}
      tone="danger"
      defaultFocus="cancel"
      destructive
      confirmLabel={copy.confirmLabel}
      warning={openName ? openGameWarning(openName) : undefined}
      active={active}
      onConfirm={() => void confirm()}
      onCancel={close}
    >
      {copy.body}
    </ConfirmDialog>
  );
}
