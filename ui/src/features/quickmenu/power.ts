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
        body: `O Windows reinicia do jeito normal.${opts.autostart ? ' O Sofá abre de novo quando você entrar.' : ''}`,
        confirmLabel: 'Reiniciar',
        icon: 'refresh',
        runningTitle: 'Reiniciando…',
        runningBody: 'O Windows está reiniciando.',
      };
    case 'Suspend':
      return {
        title: 'Suspender o PC?',
        body: 'O PC entra em suspensão. Ao acordar, o Sofá volta para esta tela.',
        confirmLabel: 'Suspender',
        icon: 'moon',
        runningTitle: 'Suspendendo…',
        runningBody: 'O PC está entrando em suspensão.',
      };
  }
}

/** Aviso de jogo aberto antes de qualquer acao de energia (CA-48). */
export const openGameWarning = (name: string) => `${name} está aberto. O progresso não salvo pode ser perdido.`;
