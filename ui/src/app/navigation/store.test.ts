import { beforeEach, describe, expect, it } from 'vitest';
import { selectTop, useNavigation } from './store';

describe('pilha de telas', () => {
  beforeEach(() => useNavigation.getState().reset());

  it('comeca na Home e a raiz nunca sai', () => {
    expect(selectTop(useNavigation.getState()).kind).toBe('home');
    expect(useNavigation.getState().pop()).toBe(false);
    expect(useNavigation.getState().stack).toHaveLength(1);
  });

  it('push e pop empilham e desempilham camadas', () => {
    const nav = useNavigation.getState();
    nav.push('quickMenu', 'card-1');
    nav.push('confirmDialog');
    expect(selectTop(useNavigation.getState()).kind).toBe('confirmDialog');
    expect(useNavigation.getState().pop()).toBe(true);
    const top = selectTop(useNavigation.getState());
    expect(top.kind).toBe('quickMenu');
    expect(top.returnFocusKey).toBe('card-1');
  });

  it('a mesma tela aberta duas vezes tem chaves distintas', () => {
    const nav = useNavigation.getState();
    nav.push('gameOptions');
    const a = selectTop(useNavigation.getState()).key;
    nav.pop();
    nav.push('gameOptions');
    expect(selectTop(useNavigation.getState()).key).not.toBe(a);
  });

  it('replace troca o topo e reset volta a raiz', () => {
    const nav = useNavigation.getState();
    nav.push('waiting');
    nav.replace('sessionPanel');
    expect(selectTop(useNavigation.getState()).kind).toBe('sessionPanel');
    nav.reset();
    expect(useNavigation.getState().stack).toHaveLength(1);
  });
});
