import { act, render, screen, waitFor } from '@testing-library/react';
import { setFocus } from '@noriginmedia/norigin-spatial-navigation';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { ScreenStack } from '@/app/ScreenStack';
import { resetSpatialNavigation } from '@/app/navigation/spatial';
import { selectTop, useNavigation } from '@/app/navigation/store';
import { useLibrary } from '@/features/library/store';
import { activateFocused, runBackHandler } from '@/shared/input/activation';
import type { GameDto, SessionDto } from '@/shared/ipc';
import { emitCoreEvent, mockCore, rejectWith, resetCore } from '@/shared/ipc/testing';
import { IDLE_SESSION, launchOverlay } from './overlay';
import { openGameName, playingFor } from './panel';
import { startSession, useSession } from './store';

const game = (id: string, name: string, over: Partial<GameDto> = {}): GameDto => ({
  id: `steam:${id}`,
  name,
  originalName: null,
  origin: 'Steam',
  cover: `cover://steam:${id}`,
  lastPlayedAt: 5,
  inProgress: false,
  ...over,
});
const MINERIO = game('1', 'Minério', { inProgress: true });
const OUTRO = game('2', 'Outro');
const session = (over: Partial<SessionDto>): SessionDto => ({ ...IDLE_SESSION, ...over });
const RUNNING = session({ state: 'Running', gameId: 'steam:1', since: Date.now() });
const press = (fn: () => unknown) => act(() => void fn());
const focusedText = () =>
  Array.from(document.querySelectorAll<HTMLElement>('[data-focused="true"]'))
    .map((e) => e.textContent)
    .join('|');
const kinds = () => useNavigation.getState().stack.map((e) => e.kind);

async function renderStack() {
  const result = render(<ScreenStack />);
  await waitFor(() => expect(document.querySelectorAll('[data-focused="true"]')).toHaveLength(1));
  return result;
}
async function openPanel() {
  await emitCoreEvent('session://panel', { open: true, outcome: 'Front' });
  await screen.findByRole('dialog', { name: 'Jogo em andamento' });
  await waitFor(() => expect(focusedText()).toBe('Voltar ao jogo'));
}

let stop: () => void = () => undefined;
beforeEach(() => {
  resetSpatialNavigation();
  useNavigation.getState().reset();
  useSession.setState({
    session: RUNNING,
    pendingGameId: null,
    failure: null,
    replace: null,
    reopen: null,
    swapTo: null,
    panelOutcome: null,
  });
  useLibrary.setState({ status: 'ready', games: [MINERIO, OUTRO], notice: null, focusGameId: null });
});
afterEach(() => {
  stop();
  resetCore();
});

describe('regras puras do painel e do fechamento', () => {
  it('Closing mostra "Fechando"; dispensado (CA-78) some, e o card o reabre', () => {
    const base = { pendingGameId: null, failure: null, replace: null };
    const closing = session({ state: 'Closing', gameId: 'steam:1' });
    expect(launchOverlay({ ...base, session: closing })).toEqual({ kind: 'closing', gameId: 'steam:1', swapTo: null });
    expect(launchOverlay({ ...base, session: { ...closing, dismissed: true } })).toBeNull();
    expect(launchOverlay({ ...base, session: { ...closing, dismissed: true }, reopen: 'Closing' })?.kind).toBe('closing');
  });

  it('CloseOverdue sempre pede a decisao, mesmo com dismissed (RN-12)', () => {
    const s = session({ state: 'CloseOverdue', gameId: 'steam:1', dismissed: true });
    expect(launchOverlay({ session: s, pendingGameId: null, failure: null, replace: null })).toEqual({
      kind: 'overdue',
      gameId: 'steam:1',
    });
  });

  it('"Jogando ha..." por faixa de tempo', () => {
    const now = 10_000_000;
    expect(playingFor(null, now)).toBe('Jogando agora');
    expect(playingFor(now - 30_000, now)).toBe('Jogando agora');
    expect(playingFor(now - 5 * 60_000, now)).toBe('Jogando há 5 min');
    expect(playingFor(now - 125 * 60_000, now)).toBe('Jogando há 2 h 5 min');
  });

  it('jogo aberto = sessao em andamento; Idle e erros de inicio nao contam (CA-48)', () => {
    const games = [MINERIO];
    expect(openGameName(RUNNING, games)).toBe('Minério');
    expect(openGameName(IDLE_SESSION, games)).toBeNull();
    expect(openGameName(session({ state: 'NotConfirmed', gameId: 'steam:1' }), games)).toBeNull();
  });
});

describe('painel do jogo em andamento (FE-09)', () => {
  it('abre ao receber session://panel, com "Voltar ao jogo" em foco (CA-32)', async () => {
    mockCore({});
    stop = startSession();
    await renderStack();
    await openPanel();
    expect(selectTop(useNavigation.getState()).kind).toBe('sessionPanel');
    expect(screen.getByRole('menuitem', { name: /Ir para a biblioteca/ })).toHaveTextContent('O jogo continua aberto');
  });

  it('resultado Failed nao abre o painel (o Sofa nao veio a frente)', async () => {
    mockCore({});
    stop = startSession();
    await renderStack();
    await emitCoreEvent('session://panel', { open: true, outcome: 'Failed' });
    expect(kinds()).toEqual(['home']);
  });

  it('jogo minimizado mostra o aviso do painel (CA-63)', async () => {
    mockCore({});
    stop = startSession();
    await renderStack();
    await emitCoreEvent('session://panel', { open: true, outcome: 'FrontAfterMinimize' });
    expect(await screen.findByText(/O jogo foi minimizado/)).toBeInTheDocument();
  });

  it('A em "Voltar ao jogo" chama session_return_to_game e fecha o painel (CA-34)', async () => {
    const core = mockCore({ session_return_to_game: null });
    stop = startSession();
    await renderStack();
    await openPanel();
    press(activateFocused);
    await waitFor(() => expect(core.count('session_return_to_game')).toBe(1));
    await waitFor(() => expect(kinds()).toEqual(['home']));
  });

  it('B faz o mesmo que "Voltar ao jogo"', async () => {
    const core = mockCore({ session_return_to_game: null });
    stop = startSession();
    await renderStack();
    await openPanel();
    press(runBackHandler);
    await waitFor(() => expect(core.count('session_return_to_game')).toBe(1));
  });

  it('ForegroundDenied: "Tentar de novo" em foco e o painel continua; Ir para a biblioteca volta a raiz', async () => {
    mockCore({ session_return_to_game: rejectWith({ code: 'ForegroundDenied' }) });
    stop = startSession();
    await renderStack();
    await openPanel();
    press(activateFocused);
    expect(await screen.findByText('O Windows não deixou voltar ao jogo. Tente de novo.')).toBeInTheDocument();
    await waitFor(() => expect(focusedText()).toBe('Tentar de novo'));
    expect(kinds()).toContain('sessionPanel');
    press(() => setFocus('dlg-cancel'));
    await waitFor(() => expect(focusedText()).toBe('Ir para a biblioteca'));
    press(activateFocused);
    await waitFor(() => expect(kinds()).toEqual(['home']));
  });

  it('"Ir para a biblioteca" fecha o painel sem tocar no jogo (CA-36)', async () => {
    const core = mockCore({});
    stop = startSession();
    await renderStack();
    await openPanel();
    press(() => setFocus('menu-library'));
    await waitFor(() => expect(focusedText()).toContain('Ir para a biblioteca'));
    press(activateFocused);
    await waitFor(() => expect(kinds()).toEqual(['home']));
    // So a carga inicial: nenhum comando de sessao depois (nao fecha, nao volta, nao minimiza).
    expect(core.calls.filter((c) => c.command.startsWith('session_') && c.command !== 'session_get')).toHaveLength(0);
  });

  it('o jogo fecha sozinho com o painel aberto: o painel some', async () => {
    mockCore({});
    stop = startSession();
    await renderStack();
    await openPanel();
    await emitCoreEvent('session://state', session({ state: 'Idle' }));
    await waitFor(() => expect(kinds()).toEqual(['home']));
  });
});

describe('fechar o jogo (CA-35, CA-78, RN-12)', () => {
  async function toConfirm() {
    await renderStack();
    await openPanel();
    press(() => setFocus('menu-close'));
    await waitFor(() => expect(focusedText()).toContain('Fechar jogo'));
    press(activateFocused);
    await screen.findByText('Fechar Minério?');
  }

  it('confirmacao abre com "Cancelar" em foco; Cancelar nao chama o nucleo', async () => {
    const core = mockCore({ session_close: null });
    stop = startSession();
    await toConfirm();
    await waitFor(() => expect(focusedText()).toBe('Cancelar'));
    expect(screen.getByText('O progresso não salvo pode ser perdido.')).toBeInTheDocument();
    press(activateFocused);
    await waitFor(() => expect(screen.queryByText('Fechar Minério?')).toBeNull());
    expect(core.count('session_close')).toBe(0);
  });

  it('confirmar chama session_close; Closing mostra "Fechando <nome>…" com "Ir para a biblioteca" em foco', async () => {
    const core = mockCore({ session_close: null, session_dismiss: null });
    stop = startSession();
    await toConfirm();
    press(() => setFocus('dlg-confirm'));
    await waitFor(() => expect(focusedText()).toBe('Fechar jogo'));
    press(activateFocused);
    await waitFor(() => expect(core.count('session_close')).toBe(1));
    await emitCoreEvent('session://state', session({ state: 'Closing', gameId: 'steam:1' }));
    expect(await screen.findByText('Fechando Minério…')).toBeInTheDocument();
    await waitFor(() => expect(focusedText()).toBe('Ir para a biblioteca'));
  });

  it('B em "Fechando" leva a biblioteca sem cancelar o fechamento; o card reabre "Fechando" e depois o "Forcar"', async () => {
    const core = mockCore({ session_dismiss: null, session_return_to_game: null, session_force_close: null });
    stop = startSession();
    await renderStack();
    await emitCoreEvent('session://state', session({ state: 'Closing', gameId: 'steam:1' }));
    await screen.findByText('Fechando Minério…');
    press(runBackHandler);
    await waitFor(() => expect(core.count('session_dismiss')).toBe(1));
    await waitFor(() => expect(screen.queryByText('Fechando Minério…')).toBeNull());
    expect(kinds()).toEqual(['home']);

    // Card "Em andamento": A nao chama session_return_to_game (InvalidState em Closing), reabre a espera.
    await act(async () => void (await useSession.getState().play(MINERIO)));
    expect(await screen.findByText('Fechando Minério…')).toBeInTheDocument();
    expect(core.count('session_return_to_game')).toBe(0);

    // 10 s sem fechar: oferece forcar, com "Cancelar" em foco.
    await emitCoreEvent('session://state', session({ state: 'CloseOverdue', gameId: 'steam:1' }));
    expect(await screen.findByText('Minério não fechou')).toBeInTheDocument();
    expect(screen.getByText('O progresso não salvo será perdido.')).toBeInTheDocument();
    await waitFor(() => expect(focusedText()).toBe('Cancelar'));
    expect(core.count('session_force_close')).toBe(0);
  });

  it('A em "Fechando" nos primeiros 600 ms e ignorado (RN-31)', async () => {
    const core = mockCore({ session_dismiss: null });
    stop = startSession();
    await renderStack();
    await emitCoreEvent('session://state', session({ state: 'Closing', gameId: 'steam:1' }));
    await screen.findByText('Fechando Minério…');
    press(activateFocused);
    expect(core.count('session_dismiss')).toBe(0);
    await act(() => new Promise((r) => setTimeout(r, 650)));
    press(activateFocused);
    await waitFor(() => expect(core.count('session_dismiss')).toBe(1));
  });

  it('CloseOverdue ao voltar pelo card: Forcar chama session_force_close', async () => {
    const core = mockCore({ session_force_close: null, session_dismiss: null });
    stop = startSession();
    await renderStack();
    await emitCoreEvent('session://state', session({ state: 'CloseOverdue', gameId: 'steam:1', dismissed: true }));
    await screen.findByText('Minério não fechou');
    press(() => setFocus('dlg-confirm'));
    await waitFor(() => expect(focusedText()).toBe('Forçar fechamento'));
    press(activateFocused);
    await waitFor(() => expect(core.count('session_force_close')).toBe(1));
  });

  it('Cancelar na oferta de forcar chama session_dismiss (a sessao volta a Running, CA-35)', async () => {
    const core = mockCore({ session_dismiss: null });
    stop = startSession();
    await renderStack();
    await emitCoreEvent('session://state', session({ state: 'CloseOverdue', gameId: 'steam:1' }));
    await screen.findByText('Minério não fechou');
    press(runBackHandler);
    await waitFor(() => expect(core.count('session_dismiss')).toBe(1));
  });

  it('forcar recusado (jogo administrador): texto proprio e Voltar devolve a sessao a Running', async () => {
    const core = mockCore({ session_force_close: rejectWith({ code: 'AccessDenied' }), session_dismiss: null });
    stop = startSession();
    await renderStack();
    await emitCoreEvent('session://state', session({ state: 'CloseOverdue', gameId: 'steam:1' }));
    await screen.findByText('Minério não fechou');
    press(() => setFocus('dlg-confirm'));
    await waitFor(() => expect(focusedText()).toBe('Forçar fechamento'));
    press(activateFocused);
    expect(await screen.findByText(/O jogo roda como administrador/)).toBeInTheDocument();
    press(activateFocused);
    await waitFor(() => expect(core.count('session_dismiss')).toBe(1));
  });

  it('jogo fechou: aviso "<nome> fechado." e nada fica na pilha', async () => {
    mockCore({ session_dismiss: null });
    stop = startSession();
    await renderStack();
    await openPanel();
    await emitCoreEvent('session://state', session({ state: 'Closing', gameId: 'steam:1' }));
    await emitCoreEvent('session://state', session({ state: 'Idle' }));
    await waitFor(() => expect(useLibrary.getState().notice).toBe('Minério fechado.'));
    await waitFor(() => expect(kinds()).toEqual(['home']));
  });
});

describe('trocar de jogo conforme CT-01', () => {
  it('confirma session_replace, mostra o proximo jogo em "Fechando" e abre a espera ao chegar a Idle', async () => {
    const core = mockCore({
      session_launch: rejectWith({ code: 'SessionAlreadyActive', params: { gameId: 'steam:1' } }),
      session_replace: null,
    });
    stop = startSession();
    await renderStack();
    await act(async () => void (await useSession.getState().play(OUTRO)));
    await screen.findByText('Minério está aberto. Fechar e abrir Outro?');
    press(() => setFocus('dlg-confirm'));
    await waitFor(() => expect(focusedText()).toBe('Fechar e abrir'));
    press(activateFocused);
    await waitFor(() => expect(core.count('session_replace')).toBe(1));

    await emitCoreEvent('session://state', session({ state: 'Closing', gameId: 'steam:1' }));
    expect(await screen.findByText(/Depois o Sofá abre Outro/)).toBeInTheDocument();
    await emitCoreEvent('session://state', session({ state: 'Idle' }));
    expect(await screen.findByText('Abrindo Outro…')).toBeInTheDocument();
  });

  it('jogo que ignora WM_CLOSE: CloseOverdue descarta a troca e oferece forcar (nada e forcado sozinho)', async () => {
    const core = mockCore({
      session_launch: rejectWith({ code: 'SessionAlreadyActive', params: { gameId: 'steam:1' } }),
      session_replace: null,
      session_force_close: null,
    });
    stop = startSession();
    await renderStack();
    await act(async () => void (await useSession.getState().play(OUTRO)));
    await screen.findByRole('alertdialog');
    press(() => setFocus('dlg-confirm'));
    await waitFor(() => expect(focusedText()).toBe('Fechar e abrir'));
    press(activateFocused);
    await waitFor(() => expect(core.count('session_replace')).toBe(1));
    await emitCoreEvent('session://state', session({ state: 'Closing', gameId: 'steam:1' }));
    await emitCoreEvent('session://state', session({ state: 'CloseOverdue', gameId: 'steam:1' }));
    expect(await screen.findByText('Minério não fechou')).toBeInTheDocument();
    expect(useSession.getState().swapTo).toBeNull();
    expect(core.count('session_force_close')).toBe(0);
  });
});
