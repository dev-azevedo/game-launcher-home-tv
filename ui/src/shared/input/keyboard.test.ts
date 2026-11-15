import { describe, expect, it, vi } from 'vitest';
import { INTENTS } from './intents';
import { createKeyboardSource, keyToIntent } from './keyboard';

describe('teclado de emergencia -> intencao (RN-07)', () => {
  it.each([
    ['ArrowUp', 'Up'],
    ['ArrowDown', 'Down'],
    ['ArrowLeft', 'Left'],
    ['ArrowRight', 'Right'],
    ['Enter', 'Confirm'],
    ['Escape', 'Back'],
    ['ContextMenu', 'Menu'],
    ['Backspace', 'Back'],
    ['F1', 'Menu'],
    ['F2', 'View'],
    ['PageUp', 'PagePrev'],
    ['PageDown', 'PageNext'],
    ['[', 'VolumeDown'],
    [']', 'VolumeUp'],
  ])('%s -> %s', (key, intent) => {
    expect(keyToIntent(key)).toBe(intent);
  });

  it('teclas de teste dos mockups nao valem no produto', () => {
    for (const key of ['v', 'V', 'm', 'F5', 'F9', 'F12', 'Tab', '-', '+']) {
      expect(keyToIntent(key)).toBeNull();
    }
  });

  it('em campo de texto so Enter, Esc e setas verticais saem do campo', () => {
    expect(keyToIntent('ArrowLeft', true)).toBeNull();
    expect(keyToIntent('ArrowRight', true)).toBeNull();
    expect(keyToIntent('Enter', true)).toBe('Confirm');
    expect(keyToIntent('Escape', true)).toBe('Back');
    expect(keyToIntent('ArrowDown', true)).toBe('Down');
  });

  it('em campo de texto os caracteres [ ] e Backspace/PgUp/PgDn/F2 ficam no campo (renomear)', () => {
    for (const key of ['[', ']', 'Backspace', 'PageUp', 'PageDown', 'F2', 'F1']) expect(keyToIntent(key, true)).toBeNull();
  });

  it('cobre toda intencao do controle, exceto a combinacao de retorno (CA-80)', () => {
    const covered = new Set(
      ['ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'Enter', 'Escape', 'F1', 'F2', 'PageUp', 'PageDown', '[', ']'].map((k) =>
        keyToIntent(k),
      ),
    );
    for (const intent of INTENTS) expect(covered.has(intent)).toBe(true);
  });

  it('a combinacao de retorno nao existe no teclado: F1 e F2 juntos nao emitem nada alem de Menu e View', () => {
    const target = document.createElement('div');
    const handler = vi.fn();
    createKeyboardSource(target).subscribe(handler);
    target.dispatchEvent(new KeyboardEvent('keydown', { key: 'F1', cancelable: true }));
    target.dispatchEvent(new KeyboardEvent('keydown', { key: 'F2', cancelable: true }));
    expect(handler.mock.calls.map((c) => c[0].intent)).toEqual(['Menu', 'View']);
  });

  it('a fonte emite a intencao com repeat e cancela o comportamento padrao', () => {
    const target = document.createElement('div');
    const handler = vi.fn();
    const off = createKeyboardSource(target).subscribe(handler);
    const down = new KeyboardEvent('keydown', { key: 'ArrowDown', repeat: true, cancelable: true });
    target.dispatchEvent(down);
    expect(handler).toHaveBeenCalledWith({ intent: 'Down', repeat: true });
    expect(down.defaultPrevented).toBe(true);
    off();
    target.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter' }));
    expect(handler).toHaveBeenCalledTimes(1);
  });

  it('ignora Ctrl/Alt+tecla (atalhos do navegador ficam fora)', () => {
    const target = document.createElement('div');
    const handler = vi.fn();
    createKeyboardSource(target).subscribe(handler);
    target.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', ctrlKey: true }));
    expect(handler).not.toHaveBeenCalled();
  });
});
