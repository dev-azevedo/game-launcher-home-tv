import { act, cleanup, render, screen, waitFor } from '@testing-library/react';
import { getCurrentFocusKey, setFocus } from '@noriginmedia/norigin-spatial-navigation';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { ScreenStack } from '@/app/ScreenStack';
import { defaultDeps, dispatchIntent } from '@/app/navigation/dispatch';
import { resetSpatialNavigation } from '@/app/navigation/spatial';
import { useNavigation } from '@/app/navigation/store';
import { stripGames } from '@/features/library/domain';
import { useLibrary } from '@/features/library/store';
import { IDLE_SESSION } from '@/features/session/overlay';
import { useSession } from '@/features/session/store';
import { activateFocused } from '@/shared/input/activation';
import type { InputIntent } from '@/shared/input/intents';
import type { GameDto } from '@/shared/ipc';
import { mockCore, resetCore } from '@/shared/ipc/testing';
import { cycleLayout, DEFAULT_LAYOUT, layoutLabel, LAYOUTS, parseLayout } from './domain';
import { LAYOUT_STORAGE_KEY, startAppearance, useAppearance } from './store';

const game = (id: string, name: string, lastPlayedAt: number | null = null): GameDto => ({
  id: `steam:${id}`,
  name,
  originalName: null,
  origin: 'Steam',
  cover: `cover://steam:${id}`,
  lastPlayedAt,
  inProgress: false,
});
const GAMES = [game('1', 'Zeta', 300), game('2', 'Alfa', 200), game('3', 'Beta'), game('4', 'Gama')];

const press = (fn: () => unknown) => act(() => void fn());
const intent = (i: InputIntent) => act(() => void dispatchIntent({ intent: i, repeat: false }, defaultDeps()));

async function renderHome() {
  render(<ScreenStack />);
  await waitFor(() => expect(document.querySelectorAll('[data-focused="true"]')).toHaveLength(1));
}

beforeEach(() => {
  window.localStorage.clear();
  startAppearance();
  resetSpatialNavigation();
  useNavigation.getState().reset();
  useSession.setState({ session: IDLE_SESSION, failure: null, replace: null });
  useLibrary.setState({ status: 'ready', games: GAMES, focusGameId: null, refreshing: false, notice: null });
});
afterEach(() => {
  // Desmonta antes de limpar o mock do nucleo: o `unlisten` dos eventos precisa dele.
  cleanup();
  resetCore();
});

describe('dominio do layout', () => {
  it('valor salvo desconhecido ou ausente vira o padrao', () => {
    expect(parseLayout('vitrine')).toBe('vitrine');
    expect(parseLayout('mosaico')).toBe('mosaico');
    expect(parseLayout('ps5')).toBe(DEFAULT_LAYOUT);
    expect(parseLayout(null)).toBe('sofa');
  });

  it('ciclo com volta nas duas direcoes', () => {
    expect(cycleLayout('sofa', 1)).toBe('vitrine');
    expect(cycleLayout('mosaico', 1)).toBe('sofa');
    expect(cycleLayout('sofa', -1)).toBe('mosaico');
    expect(LAYOUTS.map((l) => layoutLabel(l.id))).toEqual(['Sofá (padrão)', 'Vitrine', 'Mosaico']);
  });

  it('fileira da Vitrine: Recentes e depois A-Z sem repetir', () => {
    const recents = [GAMES[0]!, GAMES[1]!];
    const az = [GAMES[1]!, GAMES[2]!, GAMES[3]!, GAMES[0]!];
    expect(stripGames(recents, az).map((g) => g.name)).toEqual(['Zeta', 'Alfa', 'Beta', 'Gama']);
  });
});

describe('store do layout', () => {
  it('salva, aplica no <html> e volta ao abrir o app de novo', () => {
    expect(document.documentElement.dataset.layout).toBe('sofa');
    act(() => useAppearance.getState().setLayout('mosaico'));
    expect(window.localStorage.getItem(LAYOUT_STORAGE_KEY)).toBe('mosaico');
    expect(document.documentElement.dataset.layout).toBe('mosaico');
    useAppearance.setState({ layout: 'sofa' });
    startAppearance();
    expect(useAppearance.getState().layout).toBe('mosaico');
  });

  it('valor estranho no armazenamento abre no padrao', () => {
    window.localStorage.setItem(LAYOUT_STORAGE_KEY, '<script>');
    startAppearance();
    expect(useAppearance.getState().layout).toBe('sofa');
    expect(document.documentElement.dataset.layout).toBe('sofa');
  });
});

describe('Home por layout', () => {
  it('Sofa: duas fileiras de cartazes, sem arte de fundo', async () => {
    await renderHome();
    expect(screen.getByRole('region', { name: 'Recentes' })).toBeInTheDocument();
    expect(screen.getByRole('region', { name: 'Biblioteca' })).toBeInTheDocument();
    expect(screen.queryByTestId('backdrop')).toBeNull();
    expect(document.querySelector('[data-screen="home"]')).toHaveAttribute('data-layout', 'sofa');
  });

  it('Vitrine: uma fileira so, sem repetir jogo, com arte de fundo e botao Jogar', async () => {
    act(() => useAppearance.getState().setLayout('vitrine'));
    await renderHome();
    const row = screen.getByRole('region', { name: 'Jogos' });
    const keys = Array.from(row.querySelectorAll<HTMLElement>('[data-focus-key]')).map((e) => e.dataset.focusKey);
    expect(keys).toEqual(['r:steam:1', 'r:steam:2', 'a:steam:3', 'a:steam:4', 'all']);
    expect(screen.queryByRole('region', { name: 'Recentes' })).toBeNull();
    // Foco inicial: o ultimo jogado (mesma regra do padrao).
    await waitFor(() => expect(getCurrentFocusKey()).toBe('r:steam:1'));
    expect(screen.getByTestId('backdrop').querySelector('img')).not.toBeNull();
    expect(document.querySelector('.hero-cta')).toHaveTextContent('Jogar');
  });

  it('Mosaico: o ultimo jogado vira bloco largo e ha arte de fundo', async () => {
    act(() => useAppearance.getState().setLayout('mosaico'));
    await renderHome();
    const recents = screen.getByRole('region', { name: 'Recentes' });
    const cards = recents.querySelectorAll('.card');
    expect(cards[0]).toHaveClass('wide');
    expect(cards[1]).not.toHaveClass('wide');
    expect(screen.getByTestId('backdrop')).toBeInTheDocument();
  });

  it('trocar o layout com a Home aberta remonta a fileira e mantem o foco valido', async () => {
    await renderHome();
    act(() => useAppearance.getState().setLayout('vitrine'));
    expect(await screen.findByRole('region', { name: 'Jogos' })).toBeInTheDocument();
    await waitFor(() => expect(getCurrentFocusKey()).toMatch(/^(r|a):|^all$/));
  });
});

describe('troca pelo menu rapido e pelas configuracoes', () => {
  it('menu rapido: ←/→ no Layout trocam o layout; A avanca', async () => {
    mockCore({ volume_get: { level: 40, muted: false, available: true }, power_capabilities: {} });
    await renderHome();
    intent('Menu');
    await screen.findByRole('dialog', { name: 'Menu rápido' });
    press(() => setFocus('menu-layout'));
    await waitFor(() => expect(getCurrentFocusKey()).toBe('menu-layout'));
    intent('Right');
    expect(useAppearance.getState().layout).toBe('vitrine');
    intent('Left');
    intent('Left');
    expect(useAppearance.getState().layout).toBe('mosaico');
    expect(screen.getByRole('button', { name: /Layout/ })).toHaveTextContent('Mosaico');
    press(activateFocused);
    expect(useAppearance.getState().layout).toBe('sofa');
  });

  it('configuracoes: aba Aparencia marca o layout em uso e troca com A', async () => {
    mockCore({ settings_get: null, display_monitors: [], library_hidden: [] });
    await renderHome();
    act(() => useNavigation.getState().push('settings'));
    await screen.findByRole('tablist', { name: 'Configurações' });
    press(() => setFocus('tab-appearance'));
    await waitFor(() => expect(getCurrentFocusKey()).toBe('tab-appearance'));
    const sofa = await screen.findByRole('menuitemradio', { name: /Sofá/ });
    expect(sofa).toHaveAttribute('aria-checked', 'true');
    press(() => setFocus('menu-layout-mosaico'));
    await waitFor(() => expect(getCurrentFocusKey()).toBe('menu-layout-mosaico'));
    press(activateFocused);
    expect(useAppearance.getState().layout).toBe('mosaico');
    await waitFor(() => expect(screen.getByRole('menuitemradio', { name: /Mosaico/ })).toHaveAttribute('aria-checked', 'true'));
  });
});
