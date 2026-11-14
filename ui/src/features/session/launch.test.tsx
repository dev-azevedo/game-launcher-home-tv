import { act, render, screen, waitFor } from '@testing-library/react';
import { setFocus } from '@noriginmedia/norigin-spatial-navigation';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { ScreenStack } from '@/app/ScreenStack';
import { resetSpatialNavigation } from '@/app/navigation/spatial';
import { selectTop, useNavigation } from '@/app/navigation/store';
import { useLibrary } from '@/features/library/store';
import { activateFocused, runBackHandler } from '@/shared/input/activation';
import type { GameDto, SessionDto } from '@/shared/ipc';
import { mockCore, rejectWith, resetCore } from '@/shared/ipc/testing';
import { IDLE_SESSION, launchOverlay } from './overlay';
import { useSession } from './store';

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
const MINERIO = game('1', 'Minério');
const OUTRO = game('2', 'Outro');
const session = (over: Partial<SessionDto>): SessionDto => ({ ...IDLE_SESSION, ...over });
const press = (fn: () => unknown) => act(() => void fn());
const focusedText = () => Array.from(document.querySelectorAll<HTMLElement>('[data-focused="true"]')).map((e) => e.textContent).join('|');

/** Espera o foco inicial da Home assentar antes de abrir camadas por cima. */
async function renderStack() {
  const result = render(<ScreenStack />);
  await waitFor(() => expect(document.querySelectorAll('[data-focused="true"]')).toHaveLength(1));
  return result;
}

beforeEach(() => {
  resetSpatialNavigation();
  useNavigation.getState().reset();
  useSession.setState({ session: IDLE_SESSION, pendingGameId: null, failure: null, replace: null });
  useLibrary.setState({ status: 'ready', games: [MINERIO, OUTRO], notice: null });
});
afterEach(() => resetCore());

describe('regra da tela de sessao (launchOverlay)', () => {
  const base = { pendingGameId: null, failure: null, replace: null };

  it('Launching mostra a espera; dispensada (dismissed) nao reabre a cada evento', () => {
    const s = session({ state: 'Launching', gameId: 'steam:1' });
    expect(launchOverlay({ ...base, session: s })).toEqual({ kind: 'launching', gameId: 'steam:1' });
    expect(launchOverlay({ ...base, session: { ...s, dismissed: true } })).toBeNull();
  });

  it('Launching com elevated mostra a espera do UAC (2a etapa)', () => {
    const s = session({ state: 'Launching', gameId: 'steam:1', elevated: true });
    expect(launchOverlay({ ...base, session: s })?.kind).toBe('uacWait');
  });

  it('NeedsElevation: aviso da 1a etapa; com launchFailure, UAC recusado', () => {
    const s = session({ state: 'NeedsElevation', gameId: 'steam:1' });
    expect(launchOverlay({ ...base, session: s })?.kind).toBe('needsElevation');
    expect(launchOverlay({ ...base, session: { ...s, launchFailure: 'ElevationDenied' } })?.kind).toBe('elevationDenied');
  });

  it('Running e Idle sem pedido nao mostram nada', () => {
    expect(launchOverlay({ ...base, session: session({ state: 'Running', gameId: 'steam:1' }) })).toBeNull();
    expect(launchOverlay({ ...base, session: IDLE_SESSION })).toBeNull();
  });

  it('pedido sem resposta ainda mostra a espera (entre o A e o evento)', () => {
    expect(launchOverlay({ ...base, session: IDLE_SESSION, pendingGameId: 'steam:1' })?.kind).toBe('launching');
  });
});

describe('iniciar jogo (FE-08)', () => {
  it('"Abrindo <nome>…" com Voltar em foco; B chama session_dismiss e a UI segue o fluxo de sessao (CA-77)', async () => {
    const core = mockCore({ session_launch: null, session_dismiss: null });
    await renderStack();
    await act(async () => void (await useSession.getState().play(MINERIO)));
    act(() => useSession.getState().setSession(session({ state: 'Launching', gameId: 'steam:1' })));
    expect(await screen.findByText('Abrindo Minério…')).toBeInTheDocument();
    await waitFor(() => expect(screen.getByRole('button', { name: 'Voltar' })).toHaveAttribute('data-focused', 'true'));
    expect(selectTop(useNavigation.getState()).kind).toBe('launchFlow');

    press(runBackHandler);
    await waitFor(() => expect(core.count('session_dismiss')).toBe(1));
    expect(screen.queryByText('Abrindo Minério…')).toBeNull();
    // O jogo abre depois: vale o fluxo normal de sessao.
    act(() => useSession.getState().setSession(session({ state: 'Running', gameId: 'steam:1' })));
    expect(screen.queryByRole('alertdialog')).toBeNull();
    await waitFor(() => expect(useNavigation.getState().stack).toHaveLength(1));
  });

  it('GameFileMissing: Ocultar e Voltar, foco em Voltar; Ocultar chama game_hide (CA-28)', async () => {
    const core = mockCore({
      session_launch: rejectWith({ code: 'GameFileMissing', params: { gameId: 'steam:1' } }),
      game_hide: null,
    });
    await renderStack();
    await act(async () => void (await useSession.getState().play(MINERIO)));
    expect(await screen.findByText('Não encontramos o arquivo do jogo.')).toBeInTheDocument();
    expect(screen.getAllByText('Minério').length).toBeGreaterThan(0);
    await waitFor(() => expect(screen.getByRole('button', { name: 'Voltar' })).toHaveAttribute('data-focused', 'true'));
    press(() => setFocus('dlg-confirm'));
    await waitFor(() => expect(screen.getByRole('button', { name: 'Ocultar' })).toHaveAttribute('data-focused', 'true'));
    press(activateFocused);
    await waitFor(() => expect(core.count('game_hide')).toBe(1));
    expect(core.calls.find((c) => c.command === 'game_hide')!.args).toEqual({ gameId: 'steam:1' });
    await waitFor(() => expect(screen.queryByRole('alertdialog')).toBeNull());
  });

  it('NetworkPathBlocked: texto proprio, Ocultar e Voltar, foco em Voltar (CA-71)', async () => {
    mockCore({ session_launch: rejectWith({ code: 'NetworkPathBlocked' }) });
    await renderStack();
    await act(async () => void (await useSession.getState().play(MINERIO)));
    expect(
      await screen.findByText('Jogos em pastas de rede não são aceitos. Copie o jogo para um disco deste PC.'),
    ).toBeInTheDocument();
    await waitFor(() => expect(screen.getByRole('button', { name: 'Voltar' })).toHaveAttribute('data-focused', 'true'));
    expect(screen.getByRole('button', { name: 'Ocultar' })).toBeInTheDocument();
  });

  it('StoreNotInstalled usa params.origin para o nome da loja', async () => {
    mockCore({ session_launch: rejectWith({ code: 'StoreNotInstalled', params: { origin: 'Epic' } }) });
    await renderStack();
    await act(async () => void (await useSession.getState().play(MINERIO)));
    expect(await screen.findByText(/Não encontramos a Epic neste PC/)).toBeInTheDocument();
  });

  it('administrador em duas etapas: Continuar em foco; UAC so depois de Continuar (CA-73, RN-27)', async () => {
    const core = mockCore({ session_launch_with_uac: null, session_dismiss: null });
    await renderStack();
    act(() => useSession.getState().setSession(session({ state: 'NeedsElevation', gameId: 'steam:1' })));
    expect(await screen.findByText(/precisa de permissão de administrador para abrir/)).toBeInTheDocument();
    await waitFor(() => expect(screen.getByRole('button', { name: 'Continuar' })).toHaveAttribute('data-focused', 'true'));
    expect(core.count('session_launch_with_uac')).toBe(0);
    press(activateFocused);
    await waitFor(() => expect(core.count('session_launch_with_uac')).toBe(1));
    expect(core.calls[0]!.args).toEqual({ gameId: 'steam:1' });
  });

  it('administrador: Voltar (e B) antes do UAC leva a Idle via session_dismiss, sem lancar nada', async () => {
    const core = mockCore({ session_launch_with_uac: null, session_dismiss: null });
    await renderStack();
    act(() => useSession.getState().setSession(session({ state: 'NeedsElevation', gameId: 'steam:1' })));
    await screen.findByRole('alertdialog');
    press(runBackHandler);
    await waitFor(() => expect(core.count('session_dismiss')).toBe(1));
    expect(core.count('session_launch_with_uac')).toBe(0);
  });

  it('esperando o UAC: aviso por tras do UAC com Voltar; e UAC recusado mostra a mensagem', async () => {
    mockCore({ session_dismiss: null });
    await renderStack();
    act(() => useSession.getState().setSession(session({ state: 'Launching', gameId: 'steam:1', elevated: true })));
    expect(await screen.findByText('Minério pede permissão de administrador')).toBeInTheDocument();
    expect(screen.getByText(/Confirme no Windows com teclado ou mouse/)).toBeInTheDocument();
    act(() =>
      useSession.getState().setSession(session({ state: 'NeedsElevation', gameId: 'steam:1', launchFailure: 'ElevationDenied' })),
    );
    expect(await screen.findByText('O Windows não deu permissão para abrir Minério.')).toBeInTheDocument();
  });

  it('nao confirmado: "Tentar de novo" em foco chama session_retry (CA-29)', async () => {
    const core = mockCore({ session_retry: null });
    await renderStack();
    act(() => useSession.getState().setSession(session({ state: 'NotConfirmed', gameId: 'steam:1' })));
    expect(await screen.findByText('Não foi possível confirmar que Minério abriu.')).toBeInTheDocument();
    await waitFor(() => expect(focusedText()).toBe('Tentar de novo'));
    press(activateFocused);
    await waitFor(() => expect(core.count('session_retry')).toBe(1));
  });

  it('troca de jogo: abre com "Cancelar" em foco; so confirmar chama session_replace (CA-31)', async () => {
    const core = mockCore({
      session_launch: rejectWith({ code: 'SessionAlreadyActive', params: { gameId: 'steam:1' } }),
      session_replace: null,
    });
    await renderStack();
    await act(async () => void (await useSession.getState().play(OUTRO)));
    expect(await screen.findByText('Minério está aberto. Fechar e abrir Outro?')).toBeInTheDocument();
    await waitFor(() => expect(focusedText()).toBe('Cancelar'));
    expect(core.count('session_replace')).toBe(0);
    press(() => setFocus('dlg-confirm'));
    await waitFor(() => expect(focusedText()).toBe('Fechar e abrir'));
    press(activateFocused);
    await waitFor(() => expect(core.count('session_replace')).toBe(1));
    expect(core.calls.find((c) => c.command === 'session_replace')!.args).toEqual({ gameId: 'steam:2' });
  });

  it('troca de jogo: Cancelar nao chama o nucleo', async () => {
    const core = mockCore({
      session_launch: rejectWith({ code: 'SessionAlreadyActive', params: { gameId: 'steam:1' } }),
    });
    await renderStack();
    await act(async () => void (await useSession.getState().play(OUTRO)));
    await screen.findByRole('alertdialog');
    press(activateFocused);
    await waitFor(() => expect(screen.queryByRole('alertdialog')).toBeNull());
    expect(core.count('session_replace')).toBe(0);
  });

  it('nome com HTML aparece literal na espera (SEC-07)', async () => {
    const evil = '<img src=x onerror=alert(1)>';
    useLibrary.setState({ games: [game('9', evil)] });
    const { container } = await renderStack();
    act(() => useSession.getState().setSession(session({ state: 'Launching', gameId: 'steam:9' })));
    expect(await screen.findByText(`Abrindo ${evil}…`)).toBeInTheDocument();
    expect(container.querySelectorAll('img[onerror]')).toHaveLength(0);
  });
});
