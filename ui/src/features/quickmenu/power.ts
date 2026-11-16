import type { PowerAction } from '@/shared/ipc';
import type { IconName } from '@/shared/ui/Icon';

export interface PowerCopy {
  title: string;
  body: string;
  confirmLabel: string;
  icon: IconName;
  /** Tela depois de confirmar ("Desligando…"). */
  runningTitle: string;
  runningBody: string;
}

/** Textos de desligar, reiniciar e suspender (microcopy, Menu rapido). */
export function powerCopy(action: PowerAction, opts: { autostart: boolean }): PowerCopy {
  switch (action) {
    case 'Shutdown':
      return {
        title: 'Desligar o PC?',
        body: 'O Windows desliga do jeito normal. Se outro app tiver trabalho não salvo, o Windows avisa.',
        confirmLabel: 'Desligar',
        icon: 'power',
        runningTitle: 'Desligando…',
        runningBody: 'O Windows está encerrando.',
      };
    case 'Restart':
      return {
        title: 'Reiniciar o PC?',
        body: `O Windows reinicia do jeito normal.${opts.autostart ? ' O SofaLauncher abre de novo quando você entrar.' : ''}`,
        confirmLabel: 'Reiniciar',
        icon: 'refresh',
        runningTitle: 'Reiniciando…',
        runningBody: 'O Windows está reiniciando.',
      };
    case 'Suspend':
      return {
        title: 'Suspender o PC?',
        body: 'O PC entra em suspensão. Ao acordar, o SofaLauncher volta para esta tela.',
        confirmLabel: 'Suspender',
        icon: 'moon',
        runningTitle: 'Suspendendo…',
        runningBody: 'O PC está entrando em suspensão.',
      };
  }
}

/** Aviso de jogo aberto antes de qualquer acao de energia (CA-48). */
export const openGameWarning = (name: string) => `${name} está aberto. O progresso não salvo pode ser perdido.`;

export interface ExitCopy {
  title: string;
  confirmLabel: string;
  /** Um paragrafo por item. */
  body: string[];
  /** Aviso ambar (so com jogo aberto): o jogo segue aberto e nada se perde. */
  warning?: string;
}

/**
 * Textos de "Fechar o launcher" (CA-82, RN-35, D-58). Com "Iniciar com o Windows" desligado, reiniciar o PC
 * nao reabre o launcher: o texto cita so o atalho.
 */
export function exitCopy(opts: { gameName: string | null; autostart: boolean }): ExitCopy {
  const { gameName, autostart } = opts;
  const title = 'Fechar o launcher?';
  const confirmLabel = 'Fechar o launcher';
  if (gameName === null) {
    return {
      title,
      confirmLabel,
      body: [
        'O SofaLauncher fecha e o Windows mostra a área de trabalho.',
        autostart
          ? 'Para reabrir, use o atalho do SofaLauncher (com teclado ou mouse) ou reinicie o PC.'
          : 'Para reabrir, use o atalho do SofaLauncher (com teclado ou mouse).',
      ],
    };
  }
  return {
    title,
    confirmLabel,
    body: [
      'Só o SofaLauncher fecha.',
      autostart
        ? 'Para reabrir, use o atalho do SofaLauncher ou reinicie o PC depois de sair do jogo.'
        : 'Para reabrir, use o atalho do SofaLauncher.',
    ],
    warning: `${gameName} continuará aberto. Nenhum progresso se perde.`,
  };
}
