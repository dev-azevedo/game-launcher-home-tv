import { act, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { runPageHandler } from '@/app/navigation/pageHandler';
import { resetSpatialNavigation, startSpatialNavigation } from '@/app/navigation/spatial';
import { useFocusedGame } from '@/app/navigation/focusedGame';
import { useNavigation } from '@/app/navigation/store';
import { useSession } from '@/features/session/store';
import type { GameDto } from '@/shared/ipc';
import { emitCoreEvent, mockCore, resetCore } from '@/shared/ipc/testing';
import { AllGamesScreen } from './AllGamesScreen';
import { alphabetical, initialHomeFocus, lastPlayedLabel, recentGames } from './domain';
import { HomeScreen } from './HomeScreen';
import { startLibrary, useLibrary } from './store';

startSpatialNavigation();

const game = (id: string, name: string, lastPlayedAt: number | null = null, extra: Partial<GameDto> = {}): GameDto => ({
  id: `steam:${id}`,
  name,
  originalName: null,
  origin: 'Steam',
  cover: `cover://steam:${id}`,
  lastPlayedAt,
  inProgress: false,
  ...extra,
});

const focused = () => Array.from(document.querySelectorAll<HTMLElement>('[data-focused="true"]'));

beforeEach(() => {
  resetSpatialNavigation();
  useNavigation.getState().reset();
  useFocusedGame.getState().set(null);
  useSession.setState({ pendingGameId: null, failure: null, replace: null });
  useLibrary.setState({ status: 'loading', games: [], focusGameId: null, refreshing: false, notice: null });
});
afterEach(() => resetCore());

describe('dominio da biblioteca (RN-05, CA-22)', () => {
  const games = [game('1', 'Zeta'), game('2', 'Ágil', 100), game('3', 'beta', 300), game('4', 'Alfa', 200)];

  it('recentes: do mais recente ao mais antigo, so os ja jogados', () => {
    expect(recentGames(games).map((g) => g.name)).toEqual(['beta', 'Alfa', 'Ágil']);
  });

  it('alfabetica sem diferenciar maiusculas e acentos', () => {
    expect(alphabetical(games).map((g) => g.name)).toEqual(['Ágil', 'Alfa', 'beta', 'Zeta']);
  });

  it('foco inicial: focusGameId do nucleo, senao ultimo jogado, senao primeiro de A a Z; vazio: Adicionar', () => {
    expect(initialHomeFocus(games, 'steam:4')).toBe('r:steam:4');
    expect(initialHomeFocus(games, null)).toBe('r:steam:3');
    expect(initialHomeFocus([game('1', 'Zeta'), game('2', 'Alfa')], null)).toBe('a:steam:2');
    expect(initialHomeFocus([], null)).toBe('add');
  });

  it('ultimo jogado em dias de calendario', () => {
    const now = new Date(2026, 10, 13, 10).getTime();
    expect(lastPlayedLabel(null, now)).toBe('Nunca jogado');
    expect(lastPlayedLabel(new Date(2026, 10, 13, 1).getTime(), now)).toBe('Jogado hoje');
    expect(lastPlayedLabel(new Date(2026, 10, 12, 23).getTime(), now)).toBe('Jogado ontem');
    expect(lastPlayedLabel(new Date(2026, 10, 8, 12).getTime(), now)).toBe('Jogado há 5 dias');
  });
});

describe('store da biblioteca', () => {
  it('carrega por library_get e segue library://changed, sem polling', async () => {
    const core = mockCore({ library_get: { games: [game('1', 'Alfa')], focusGameId: null } });
    const stop = startLibrary();
    await waitFor(() => expect(useLibrary.getState().status).toBe('ready'));
    await emitCoreEvent('library://changed', { games: [game('1', 'Alfa'), game('2', 'Beta')], focusGameId: 'steam:2' });
    expect(useLibrary.getState().games).toHaveLength(2);
    expect(useLibrary.getState().focusGameId).toBe('steam:2');
    expect(core.count('library_get')).toBe(1);
    stop();
  });

  it('library_get falha: biblioteca vazia sem tela de erro (CA-18)', async () => {
    mockCore({});
    await useLibrary.getState().load();
    expect(useLibrary.getState().status).toBe('error');
    expect(useLibrary.getState().games).toEqual([]);
  });

  it('refresh: chip ate o evento chegar, depois aviso de resultado', async () => {
    mockCore({ library_refresh: null });
    await useLibrary.getState().refresh();
    expect(useLibrary.getState().refreshing).toBe(true);
    useLibrary.getState().apply({ games: [game('1', 'Alfa')], focusGameId: null });
    expect(useLibrary.getState().refreshing).toBe(false);
    expect(useLibrary.getState().notice).toBe('Biblioteca atualizada.');
  });
});

describe('HomeScreen', () => {
  it('biblioteca vazia: "Adicionar jogo" em foco (CA-23)', async () => {
    useLibrary.setState({ status: 'ready' });
    render(<HomeScreen active />);
    expect(screen.getByText('Nenhum jogo por aqui')).toBeInTheDocument();
    await waitFor(() => expect(screen.getByRole('button', { name: 'Adicionar jogo' })).toHaveAttribute('data-focused', 'true'));
    expect(focused()).toHaveLength(1);
  });

  it('cheia: foco inicial no ultimo jogado, heroi com nome inteiro e um unico foco (CA-22, CA-55)', async () => {
    useLibrary.setState({
      status: 'ready',
      games: [game('1', 'Zeta'), game('2', 'Velho', 100), game('3', 'Recente com um nome bem longo', 900)],
    });
    render(<HomeScreen active />);
    await waitFor(() => expect(focused()).toHaveLength(1));
    expect(focused()[0]!.dataset.focusKey).toBe('r:steam:3');
    await waitFor(() => expect(document.querySelector('.hero .name-big')).toHaveTextContent('Recente com um nome bem longo'));
    expect(screen.getByRole('button', { name: /Zeta, Steam/ })).toBeInTheDocument();
    expect(screen.getByText('Todos os jogos')).toBeInTheDocument();
  });

  it('capas com carga preguicosa e dimensoes fixas; nome malicioso aparece literal (SEC-07)', () => {
    const evil = '<img src=x onerror=alert(1)>';
    useLibrary.setState({ status: 'ready', games: [game('1', evil)] });
    const { container } = render(<HomeScreen active />);
    expect(container.querySelectorAll('img[onerror]')).toHaveLength(0);
    expect(screen.getAllByText(evil).length).toBeGreaterThan(0);
    const cover = container.querySelector('img.cover-img');
    expect(cover).toHaveAttribute('loading', 'lazy');
    expect(cover).toHaveAttribute('width');
  });

  it('jogo em andamento mostra o selo (CA-36)', () => {
    useLibrary.setState({ status: 'ready', games: [game('1', 'Alfa', 10, { inProgress: true })] });
    render(<HomeScreen active />);
    expect(screen.getAllByText('Em andamento').length).toBeGreaterThan(0);
  });

  it('A no card inicia o jogo; A no jogo em andamento volta a ele (CA-26, CA-36)', async () => {
    const core = mockCore({ session_launch: null, session_return_to_game: null });
    useLibrary.setState({ status: 'ready', games: [game('1', 'Alfa', 10), game('2', 'Beta', 5, { inProgress: true })] });
    await act(async () => {
      await useSession.getState().play(useLibrary.getState().games[0]!);
      useSession.setState({ pendingGameId: null }); // o nucleo ja respondeu (evento de estado)
      await useSession.getState().play(useLibrary.getState().games[1]!);
    });
    expect(core.calls.map((c) => c.command)).toEqual(['session_launch', 'session_return_to_game']);
    expect(core.calls[0]!.args).toEqual({ gameId: 'steam:1' });
  });

  it('LB/RB: RB desce para a Biblioteca e LB sobe para Recentes', async () => {
    useLibrary.setState({ status: 'ready', games: [game('1', 'Alfa', 10), game('2', 'Beta')] });
    render(<HomeScreen active />);
    await waitFor(() => expect(focused()[0]?.dataset.focusKey).toBe('r:steam:1'));
    await waitFor(() => expect(document.querySelector('.hero .name-big')).toHaveTextContent('Alfa'));
    await act(async () => void (await new Promise((r) => setTimeout(r, 100))));
    act(() => void runPageHandler(1, 1));
    await waitFor(() => expect(focused()[0]?.dataset.focusKey?.startsWith('a:')).toBe(true));
    await act(async () => void (await new Promise((r) => setTimeout(r, 150))));
    act(() => void runPageHandler(-1, 1));
    await waitFor(() => expect(focused()[0]?.dataset.focusKey).toBe('r:steam:1'));
  });
});

describe('AllGamesScreen', () => {
  const many = Array.from({ length: 500 }, (_, i) => game(String(i), `Jogo ${String(i).padStart(3, '0')}`));

  it('500 jogos: so as linhas perto da tela ficam montadas, com um unico foco', async () => {
    useLibrary.setState({ status: 'ready', games: many });
    render(<AllGamesScreen active />);
    await waitFor(() => expect(focused()).toHaveLength(1));
    const mounted = document.querySelectorAll('[data-game-card]').length;
    expect(mounted).toBeGreaterThan(0);
    expect(mounted).toBeLessThan(120);
    expect(screen.getByText('500 jogos, de A a Z')).toBeInTheDocument();
  });

  it('LB/RB pulam 3 linhas (21 jogos) e o destino ja esta montado', async () => {
    useLibrary.setState({ status: 'ready', games: many });
    render(<AllGamesScreen active />);
    await waitFor(() => expect(focused()[0]?.dataset.focusKey).toBe('g:steam:0'));
    act(() => void runPageHandler(1, 3));
    await waitFor(() => expect(focused()[0]?.dataset.focusKey).toBe('g:steam:21'));
    act(() => void runPageHandler(-1, 3));
    await waitFor(() => expect(focused()[0]?.dataset.focusKey).toBe('g:steam:0'));
  });

  it('ate 60 jogos monta todos', async () => {
    useLibrary.setState({ status: 'ready', games: many.slice(0, 50) });
    render(<AllGamesScreen active />);
    await waitFor(() => expect(document.querySelectorAll('[data-game-card]')).toHaveLength(50));
  });
});
