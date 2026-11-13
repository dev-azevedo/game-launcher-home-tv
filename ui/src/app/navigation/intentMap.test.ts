import { describe, expect, it } from 'vitest';
import { INTENTS } from '@/shared/input/intents';
import { resolveIntent } from './intentMap';
import { SCREEN_KINDS } from './screens';

describe('intencao -> acao por tela', () => {
  it('a Home ignora B (CA-10)', () => {
    expect(resolveIntent('home', 'Back')).toBeNull();
  });

  it('as demais telas voltam com B (CA-10)', () => {
    for (const kind of SCREEN_KINDS.filter((k) => k !== 'home')) {
      expect(resolveIntent(kind, 'Back')).toEqual({ type: 'back' });
    }
  });

  it('Home: A ativa, View abre opcoes, Menu abre o menu rapido, LB/RB pulam 1 linha', () => {
    expect(resolveIntent('home', 'Confirm')).toEqual({ type: 'activate' });
    expect(resolveIntent('home', 'View')).toEqual({ type: 'open', screen: 'gameOptions' });
    expect(resolveIntent('home', 'Menu')).toEqual({ type: 'open', screen: 'quickMenu' });
    expect(resolveIntent('home', 'PageNext')).toEqual({ type: 'page', delta: 1, rows: 1 });
  });

  it('Todos os jogos: LB/RB pulam 3 linhas', () => {
    expect(resolveIntent('allGames', 'PagePrev')).toEqual({ type: 'page', delta: -1, rows: 3 });
  });

  it('gatilhos de volume so valem na Home e em Todos os jogos (CA-65)', () => {
    for (const kind of SCREEN_KINDS) {
      const expected = kind === 'home' || kind === 'allGames';
      expect(resolveIntent(kind, 'VolumeUp') !== null).toBe(expected);
      expect(resolveIntent(kind, 'VolumeDown') !== null).toBe(expected);
    }
  });

  it('menu rapido: Menu fecha; no painel do jogo Menu abre o menu rapido', () => {
    expect(resolveIntent('quickMenu', 'Menu')).toEqual({ type: 'back' });
    expect(resolveIntent('sessionPanel', 'Menu')).toEqual({ type: 'open', screen: 'quickMenu' });
  });

  it('dialogos nao abrem outras telas por View/Menu', () => {
    expect(resolveIntent('confirmDialog', 'Menu')).toBeNull();
    expect(resolveIntent('confirmDialog', 'View')).toBeNull();
  });

  it('tela de espera so aceita A e B (RN-31)', () => {
    const accepted = INTENTS.filter((i) => resolveIntent('waiting', i) !== null);
    expect([...accepted].sort()).toEqual(['Back', 'Confirm']);
  });

  it('direcoes viram navegacao', () => {
    expect(resolveIntent('settings', 'Left')).toEqual({ type: 'navigate', direction: 'left' });
  });
});
