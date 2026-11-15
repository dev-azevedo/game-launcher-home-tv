import { useEffect, useMemo, useRef } from 'react';
import { selectTop, useNavigation } from '@/app/navigation/store';
import { gameName } from '@/features/library/domain';
import { useLibrary } from '@/features/library/store';
import type { GameDto } from '@/shared/ipc';
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

const nameOf = (games: readonly GameDto[], id: string) => gameName(games, id, 'o jogo');

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

  if (overlay.kind === 'closing') {
    const swap = overlay.swapTo ? nameOf(games, overlay.swapTo) : null;
    return (
      <WaitScreen
        title={`Fechando ${nameOf(games, overlay.gameId)}…`}
        buttonLabel="Ir para a biblioteca"
        // CA-78: A ou B levam a biblioteca; o pedido de fechamento continua no nucleo.
        onLeave={() => {
          void s().dismiss();
          s().showLibrary();
        }}
      >
        {swap ? `Esperando o jogo fechar. Depois o Sofá abre ${swap}.` : 'Esperando o jogo fechar. Pode levar alguns segundos.'}
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
      defaultFocus={spec.defaultFocus}
      cancelLabel={spec.cancel.label}
      confirmLabel={spec.confirm?.label}
      destructive={spec.confirm?.destructive}
      active={active}
      warning={spec.warning}
      onCancel={() => run(spec.cancel.action)}
      onConfirm={spec.confirm ? () => run(spec.confirm!.action) : undefined}
    >
      {spec.body}
    </ConfirmDialog>
  );
}
