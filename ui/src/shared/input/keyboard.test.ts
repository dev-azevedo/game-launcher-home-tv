import { describe, expect, it, vi } from 'vitest';
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
    ['F1', 'Menu'],
  ])('%s -> %s', (key, intent) => {
    expect(keyToIntent(key)).toBe(intent);
  });

  it('teclas de teste dos mockups nao valem no produto', () => {
    for (const key of ['v', 'V', 'F2', 'm', 'PageUp', 'PageDown', '[', ']', 'F5', 'F12']) {
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
