import { act, cleanup, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ButtonHints } from '@/shared/ui/ButtonHints';
import { FakeIntentSource, mergeSources } from './intents';
import { createKeyboardSource } from './keyboard';
import { useInputMode, withInputMode } from './mode';

const HINTS = [
  { glyphs: ['A' as const], label: 'Jogar' },
  { glyphs: ['view' as const], label: 'Opções' },
  { glyphs: ['menu' as const], label: 'Menu' },
  { glyphs: ['LT' as const, 'RT' as const], label: 'Volume' },
  { glyphs: ['B' as const], label: 'Voltar' },
];

const press = (key: string) => act(() => void window.dispatchEvent(new KeyboardEvent('keydown', { key })));

beforeEach(() => useInputMode.setState({ mode: 'gamepad' }));
afterEach(() => cleanup());

describe('modo de entrada', () => {
  it('a ultima fonte usada vale: teclado e depois controle', () => {
    const pad = new FakeIntentSource();
    const handler = vi.fn();
    const off = mergeSources(withInputMode(pad, 'gamepad'), withInputMode(createKeyboardSource(window), 'keyboard')).subscribe(
      handler,
    );
    expect(useInputMode.getState().mode).toBe('gamepad');
    press('ArrowRight');
    expect(useInputMode.getState().mode).toBe('keyboard');
    expect(handler).toHaveBeenLastCalledWith({ intent: 'Right', repeat: false });
    act(() => pad.emit('Down'));
    expect(useInputMode.getState().mode).toBe('gamepad');
    // Tecla sem acao no launcher nao troca o modo.
    press('x');
    expect(useInputMode.getState().mode).toBe('gamepad');
    off();
  });
});

describe('dicas de botao por modo', () => {
  it('controle: botoes A, B e gatilhos', () => {
    const { container } = render(<ButtonHints hints={HINTS} />);
    expect(container.querySelector('kbd')).toBeNull();
    expect(screen.getByText('A')).toBeInTheDocument();
    expect(screen.getByText('LT')).toBeInTheDocument();
    expect(screen.queryByText('Mover')).toBeNull();
  });

  it('teclado: teclas de cada acao e "Setas · Mover" na frente', () => {
    useInputMode.setState({ mode: 'keyboard' });
    const { container } = render(<ButtonHints hints={HINTS} />);
    const keys = Array.from(container.querySelectorAll('kbd')).map((k) => k.textContent);
    expect(keys).toEqual(['Setas', 'Enter', 'F2', 'F1', '[', ']', 'Esc']);
    expect(container.querySelector('.hint')?.textContent).toBe('SetasMover');
  });

  it('troca na hora quando o modo muda', () => {
    const { container } = render(<ButtonHints hints={HINTS} />);
    expect(container.querySelector('kbd')).toBeNull();
    act(() => useInputMode.getState().setMode('keyboard'));
    expect(screen.getByText('Enter')).toBeInTheDocument();
    act(() => useInputMode.getState().setMode('gamepad'));
    expect(screen.queryByText('Enter')).toBeNull();
  });

  it('tela que ja mostra o D-pad nao ganha "Mover" repetido', () => {
    useInputMode.setState({ mode: 'keyboard' });
    const { container } = render(<ButtonHints hints={[{ glyphs: ['dpad'], label: 'Ajustar volume' }]} />);
    expect(Array.from(container.querySelectorAll('.hint')).map((h) => h.textContent)).toEqual(['SetasAjustar volume']);
  });
});
