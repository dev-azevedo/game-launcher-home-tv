import type { InputIntent, IntentEvent, IntentSource } from './intents';

/** Teclado de emergencia (RN-07): setas = D-pad, Enter = A, Esc = B, tecla de aplicativo ou F1 = Menu. */
const KEY_TO_INTENT: Record<string, InputIntent> = {
  ArrowUp: 'Up',
  ArrowDown: 'Down',
  ArrowLeft: 'Left',
  ArrowRight: 'Right',
  Enter: 'Confirm',
  Escape: 'Back',
  ContextMenu: 'Menu',
  F1: 'Menu',
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
