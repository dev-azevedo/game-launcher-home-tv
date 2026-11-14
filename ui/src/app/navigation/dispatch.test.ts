import { describe, expect, it, vi } from 'vitest';
import { dispatchIntent, type DispatchDeps } from './dispatch';
import type { ScreenKind } from './screens';

function setup(kind: ScreenKind, over: Partial<DispatchDeps> = {}) {
  const deps: DispatchDeps = {
    top: () => ({ key: `${kind}#1`, kind }),
    push: vi.fn(),
    focusedGameId: () => 'steam:620',
    pop: vi.fn(() => true),
    moveFocus: vi.fn(),
    activate: vi.fn(() => true),
    runBackHandler: vi.fn(() => false),
    onScreenAction: vi.fn(),
    ...over,
  };
  return deps;
}

describe('dispatchIntent', () => {
  it('direcao move o foco, inclusive com repeticao (CA-07)', () => {
    const deps = setup('home');
    dispatchIntent({ intent: 'Right', repeat: true }, deps);
    expect(deps.moveFocus).toHaveBeenCalledWith('right');
  });

  it('A e atalhos nao repetem', () => {
    const deps = setup('home');
    dispatchIntent({ intent: 'Confirm', repeat: true }, deps);
    dispatchIntent({ intent: 'Menu', repeat: true }, deps);
    expect(deps.activate).not.toHaveBeenCalled();
    expect(deps.push).not.toHaveBeenCalled();
  });

  it('A ativa o item em foco', () => {
    const deps = setup('confirmDialog');
    dispatchIntent({ intent: 'Confirm', repeat: false }, deps);
    expect(deps.activate).toHaveBeenCalledOnce();
  });

  it('B na Home nao desempilha nem executa nada', () => {
    const deps = setup('home');
    expect(dispatchIntent({ intent: 'Back', repeat: false }, deps)).toBeNull();
    expect(deps.pop).not.toHaveBeenCalled();
  });

  it('B desempilha quando a camada nao registrou tratamento proprio', () => {
    const deps = setup('settings');
    dispatchIntent({ intent: 'Back', repeat: false }, deps);
    expect(deps.pop).toHaveBeenCalledOnce();
  });

  it('B usa o tratamento da camada do topo quando existe (sem desempilhar)', () => {
    const deps = setup('waiting', { runBackHandler: vi.fn(() => true) });
    dispatchIntent({ intent: 'Back', repeat: false }, deps);
    expect(deps.runBackHandler).toHaveBeenCalled();
    expect(deps.pop).not.toHaveBeenCalled();
  });

  it('Menu na Home abre o menu rapido; View abre opcoes', () => {
    const deps = setup('home');
    dispatchIntent({ intent: 'Menu', repeat: false }, deps);
    dispatchIntent({ intent: 'View', repeat: false }, deps);
    expect(deps.push).toHaveBeenNthCalledWith(1, 'quickMenu');
    expect(deps.push).toHaveBeenNthCalledWith(2, 'gameOptions', { gameId: 'steam:620' });
  });

  it('View sem jogo em foco (ex.: card "Todos os jogos") nao abre opcoes', () => {
    const deps = setup('home', { focusedGameId: () => null });
    expect(dispatchIntent({ intent: 'View', repeat: false }, deps)).toBeNull();
    expect(deps.push).not.toHaveBeenCalled();
  });

  it('volume e pagina vao para a tela; ignorados em dialogos (CA-65)', () => {
    const home = setup('home');
    dispatchIntent({ intent: 'VolumeUp', repeat: false }, home);
    dispatchIntent({ intent: 'PageNext', repeat: false }, home);
    expect(home.onScreenAction).toHaveBeenCalledTimes(2);

    const dialog = setup('confirmDialog');
    expect(dispatchIntent({ intent: 'VolumeUp', repeat: false }, dialog)).toBeNull();
    expect(dialog.onScreenAction).not.toHaveBeenCalled();
  });
});
