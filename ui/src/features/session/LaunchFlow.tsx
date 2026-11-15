import { useEffect, useMemo, useRef } from 'react';
import { selectTop, useNavigation } from '@/app/navigation/store';
import { useLibrary } from '@/features/library/store';
import { ConfirmDialog } from '@/shared/ui/ConfirmDialog';
import { WaitScreen } from '@/shared/ui/WaitScreen';
import { launchOverlay, overlayDialog, overlayWait, type DialogAction, type LaunchOverlay } from './overlay';
import { useSession } from './store';

/** Tela de sessao atual (ou `null`), derivada do estado do nucleo e do ultimo comando. */
export function useLaunchOverlay(): LaunchOverlay | null {
  const session = useSession((s) => s.session);
  const pendingGameId = useSession((s) => s.pendingGameId);
  const failure = useSession((s) => s.failure);
  const replace = useSession((s) => s.replace);
  const reopen = useSession((s) => s.reopen);
  const swapTo = useSession((s) => s.swapTo);
  return useMemo(
    () => launchOverlay({ session, pendingGameId, failure, replace, reopen, swapTo }),
    [session, pendingGameId, failure, replace, reopen, swapTo],
  );
}

/**
 * Mantem uma camada `launchFlow` na pilha enquanto ha tela de sessao, para ela ser a camada do
 * topo (so ela recebe foco e a intencao de A e B). Some quando o nucleo ou o usuario a encerram.
 */
export function useLaunchFlowEntry(): void {
  const present = useLaunchOverlay() !== null;
  const stack = useNavigation((s) => s.stack);
  const keyRef = useRef<string | null>(null);
  useEffect(() => {
    const nav = useNavigation.getState();
    // A camada pode ter saido da pilha por fora (ex.: "Ir para a biblioteca" volta a raiz): repoe se ainda ha tela.
    const inStack = keyRef.current !== null && stack.some((e) => e.key === keyRef.current);
    if (present && !inStack) {
      nav.push('launchFlow');
      keyRef.current = selectTop(useNavigation.getState()).key;
    } else if (!present && keyRef.current) {
      nav.remove(keyRef.current);
      keyRef.current = null;
    }
  }, [present, stack]);
}

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

  const wait = overlayWait(overlay, games);
  if (wait) {
    return (
      <WaitScreen
        title={wait.title}
        buttonLabel={wait.buttonLabel}
        spinLabel={wait.spinLabel}
        cover={wait.cover}
        icon={wait.icon}
        // CA-78: no fechamento, A ou B levam a biblioteca; o pedido de fechamento continua no nucleo.
        onLeave={() => {
          void s().dismiss();
          if (wait.leave === 'library') s().showLibrary();
        }}
      >
        {wait.body}
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
        if (overlay.kind === 'failure') {
          store.closeFailure();
          // Forcar recusado (jogo elevado): o jogo segue aberto, a sessao volta a Running.
          if (overlay.failure.command === 'session_force_close') void store.dismiss();
        } else void store.dismiss();
        break;
      case 'keepRunning':
        void store.dismiss();
        break;
      case 'force':
        void store.forceClose();
        break;
      case 'cancel':
        store.cancelReplace();
        break;
      case 'library':
        store.showLibrary();
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
      // Outro dialogo no mesmo lugar (ex.: erro depois de "Forcar fechamento"): remonta para o foco ir ao padrao dele.
      key={overlay.kind === 'failure' ? `failure:${overlay.failure.error.code}` : overlay.kind}
      title={spec.title}
      tone={spec.tone}
      icon={spec.icon}
      cover={spec.cover}
      defaultFocus={spec.defaultFocus}
      cancelLabel={spec.cancel.label}
      confirmLabel={spec.confirm?.label}
      destructive={spec.confirm?.destructive}
      confirmKind={spec.confirm?.kind}
      confirmIcon={spec.confirm?.icon}
      active={active}
      warning={spec.warning}
      onCancel={() => run(spec.cancel.action)}
      onConfirm={spec.confirm ? () => run(spec.confirm!.action) : undefined}
    >
      {spec.body}
    </ConfirmDialog>
  );
}
