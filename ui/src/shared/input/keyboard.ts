import type { InputIntent, IntentEvent, IntentSource } from './intents';

/**
 * Teclado de emergencia (RN-07, CA-80, D-45): cada acao do controle tem uma tecla, menos a combinacao
 * de retorno (View + Menu), que so existe no controle.
 *
 * | Tecla                    | Intencao             | Controle                       |
 * | Setas                    | Up/Down/Left/Right   | D-pad                          |
 * | Enter                    | Confirm              | A                              |
 * | Esc ou Backspace         | Back                 | B                              |
 * | F1 ou tecla de aplicativo| Menu                 | Menu (menu rapido)             |
 * | F2                       | View                 | View (opcoes do jogo)          |
 * | PgUp / PgDn              | PagePrev / PageNext  | LB / RB (pular linhas)         |
 * | [ / ]                    | VolumeDown / VolumeUp| LT / RT (volume)               |
 */
const KEY_TO_INTENT: Record<string, InputIntent> = {
  ArrowUp: 'Up',
  ArrowDown: 'Down',
  ArrowLeft: 'Left',
  ArrowRight: 'Right',
  Enter: 'Confirm',
  Escape: 'Back',
  Backspace: 'Back',
  ContextMenu: 'Menu',
  F1: 'Menu',
  F2: 'View',
  PageUp: 'PagePrev',
  PageDown: 'PageNext',
  '[': 'VolumeDown',
  ']': 'VolumeUp',
};

/** Dentro de um campo de texto so Enter, Esc e setas verticais saem do campo (handoff 9). */
const TEXT_FIELD_KEYS = new Set(['Enter', 'Escape', 'ArrowUp', 'ArrowDown']);

export function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable;
}

export function keyToIntent(key: string, inTextField = false): InputIntent | null {
  if (inTextField && !TEXT_FIELD_KEYS.has(key)) return null;
  return KEY_TO_INTENT[key] ?? null;
}

export function createKeyboardSource(target: Window | HTMLElement = window): IntentSource {
  return {
    subscribe(handler: (event: IntentEvent) => void) {
      const listener = (e: Event) => {
        const ke = e as KeyboardEvent;
        if (ke.ctrlKey || ke.altKey || ke.metaKey) return;
        const intent = keyToIntent(ke.key, isEditableTarget(ke.target));
        if (!intent) return;
        // Evita rolagem, ajuda do navegador (F1) e menu de contexto.
        ke.preventDefault();
        handler({ intent, repeat: ke.repeat });
      };
      target.addEventListener('keydown', listener);
      return () => target.removeEventListener('keydown', listener);
    },
  };
}
