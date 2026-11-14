import { useEffect, useMemo, useRef } from 'react';
import { selectTop, useNavigation } from '@/app/navigation/store';
import { useLibrary } from '@/features/library/store';
import { ConfirmDialog } from '@/shared/ui/ConfirmDialog';
import { WaitScreen } from '@/shared/ui/WaitScreen';
import { launchOverlay, overlayDialog, type DialogAction, type LaunchOverlay } from './overlay';
import { useSession } from './store';

/** Tela de sessao atual (ou `null`), derivada do estado do nucleo e do ultimo comando. */
export function useLaunchOverlay(): LaunchOverlay | null {
  const session = useSession((s) => s.session);
  const pendingGameId = useSession((s) => s.pendingGameId);
  const failure = useSession((s) => s.failure);
  const replace = useSession((s) => s.replace);
  return useMemo(
    () => launchOverlay({ session, pendingGameId, failure, replace }),
    [session, pendingGameId, failure, replace],
  );
}

/**
 * Mantem uma camada `launchFlow` na pilha enquanto ha tela de sessao, para ela ser a camada do
 * topo (so ela recebe foco e a intencao de A e B). Some quando o nucleo ou o usuario a encerram.
 */
export function useLaunchFlowEntry(): void {
  const present = useLaunchOverlay() !== null;
  const keyRef = useRef<string | null>(null);
  useEffect(() => {
    const nav = useNavigation.getState();
    if (present && !keyRef.current) {
      nav.push('launchFlow');
      keyRef.current = selectTop(useNavigation.getState()).key;
    } else if (!present && keyRef.current) {
      nav.remove(keyRef.current);
      keyRef.current = null;
    }
  }, [present]);
}

const nameOf = (games: readonly { id: string; name: string }[], id: string) =>
  games.find((g) => g.id === id)?.name ?? 'o jogo';

/**
 * Camada de inicio de jogo (FE-08): "Abrindo <nome>…" sobre WaitScreen (CA-77), aviso de
 * administrador em duas etapas (CA-73), erros do contrato (CA-28, CA-71), nao confirmado (CA-29)
 * e troca de jogo (CA-31). "Voltar" e B nunca cancelam o inicio (RN-31).
 */
export function LaunchFlow({ active }: { active: boolean }) {
  const overlay = useLaunchOverlay();
  const games = useLibrary((s) => s.games);
  const s = useSession.getState;

  if (!overlay) return null;

  if (overlay.kind === 'launching') {
    return (
      <WaitScreen title={`Abrindo ${nameOf(games, overlay.gameId)}…`} buttonLabel="Voltar" onLeave={() => void s().dismiss()}>
        Aguarde. O jogo vai aparecer na frente.
      </WaitScreen>
    );
  }
  if (overlay.kind === 'uacWait') {
    return (
      <WaitScreen
        title={`${nameOf(games, overlay.gameId)} pede permissão de administrador`}
        buttonLabel="Voltar"
        onLeave={() => void s().dismiss()}
      >
        Confirme no Windows com teclado ou mouse. A combinação de retorno pode não funcionar neste jogo.
      </WaitScreen>
    );
  }

  const spec = overlayDialog(overlay, games);
  if (!spec) return null;

  const run = (action: DialogAction) => {
    const store = s();
    switch (action) {
      case 'back':
        // Erro de comando: so fecha. Estados do nucleo (administrador, nao confirmado): Idle de verdade.
        if (overlay.kind === 'failure') store.closeFailure();
        else void store.dismiss();
        break;
      case 'cancel':
        store.cancelReplace();
        break;
      case 'library':
        store.closeFailure();
        break;
      case 'hide':
        void store.hideFailureGame();
        break;
      case 'continue':
        void store.continueWithUac();
        break;
      case 'replace':
        void store.confirmReplace();
        break;
      case 'retry':
        if (overlay.kind === 'failure') {
          store.closeFailure();
          void store.returnToGame();
        } else {
          void store.retry();
        }
        break;
    }
  };

  return (
    <ConfirmDialog
      title={spec.title}
      tone={spec.tone}
      defaultFocus={spec.defaultFocus}
      cancelLabel={spec.cancel.label}
      confirmLabel={spec.confirm?.label}
      destructive={spec.confirm?.destructive}
      active={active}
      onCancel={() => run(spec.cancel.action)}
      onConfirm={spec.confirm ? () => run(spec.confirm!.action) : undefined}
    >
      {spec.body}
    </ConfirmDialog>
  );
}
