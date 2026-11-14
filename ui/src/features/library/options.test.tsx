import { act, render, screen, waitFor } from '@testing-library/react';
import { setFocus } from '@noriginmedia/norigin-spatial-navigation';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { ScreenStack } from '@/app/ScreenStack';
import { resetSpatialNavigation } from '@/app/navigation/spatial';
import { selectTop, useNavigation } from '@/app/navigation/store';
import { useSession } from '@/features/session/store';
import { activateFocused, runBackHandler } from '@/shared/input/activation';
import type { GameDto } from '@/shared/ipc';
import { mockCore, rejectWith, resetCore } from '@/shared/ipc/testing';
import { useLibrary } from './store';

const game = (over: Partial<GameDto> = {}): GameDto => ({
  id: 'steam:1',
  name: 'Vale dos Ventos',
  originalName: null,
  origin: 'Steam',
  cover: 'cover://steam:1',
  lastPlayedAt: 5,
  inProgress: false,
  ...over,
});

const focusedLabel = () => document.querySelector<HTMLElement>('[data-focused="true"]')?.textContent ?? '';
const press = (fn: () => unknown) => act(() => void fn());

async function openOptions(g: GameDto, first = 'Jogar') {
  useLibrary.setState({ status: 'ready', games: [g, { ...g, id: 'steam:2', name: 'Outro', lastPlayedAt: null }] });
  render(<ScreenStack />);
  await waitFor(() => expect(document.querySelector('[data-focused="true"]')).not.toBeNull());
  act(() => useNavigation.getState().push('gameOptions', undefined, { gameId: g.id }));
  await screen.findByRole('menu');
  await waitFor(() => expect(focusedLabel()).toContain(first));
}

beforeEach(() => {
  resetSpatialNavigation();
  useNavigation.getState().reset();
  useSession.setState({ pendingGameId: null, failure: null, replace: null });
  useLibrary.setState({ notice: null });
});
afterEach(() => resetCore());

describe('opcoes do jogo (CA-79, RN-33)', () => {
  it('abre com "Jogar" primeiro e em foco; sem "Restaurar" com nome original', async () => {
    await openOptions(game());
    const labels = screen.getAllByRole('menuitem').map((e) => e.querySelector('.mi-label')?.textContent);
    expect(labels).toEqual(['Jogar', 'Renomear', 'Ocultar']);
    expect(document.querySelectorAll('[data-focused="true"]')).toHaveLength(1);
  });

  it('"Restaurar nome original" so aparece com nome alterado (CA-62)', async () => {
    await openOptions(game({ originalName: 'Vale do Vento' }));
    expect(screen.getByText('Restaurar nome original')).toBeInTheDocument();
  });

  it('"Jogar" faz o mesmo que A no card: session_launch', async () => {
    const core = mockCore({ session_launch: null });
    await openOptions(game());
    press(activateFocused);
    await waitFor(() => expect(core.count('session_launch')).toBe(1));
    expect(core.calls[0]!.args).toEqual({ gameId: 'steam:1' });
    // A lista fecha; o que fica por cima e a espera "Abrindo…" (FE-08), nao a lista.
    expect(useNavigation.getState().stack.some((e) => e.kind === 'gameOptions')).toBe(false);
  });

  it('jogo em andamento: primeiro item e "Voltar ao jogo"', async () => {
    const core = mockCore({ session_return_to_game: null });
    await openOptions(game({ inProgress: true }), 'Voltar ao jogo');
    press(activateFocused);
    await waitFor(() => expect(core.count('session_return_to_game')).toBe(1));
  });

  it('B fecha a lista', async () => {
    await openOptions(game());
    press(runBackHandler);
    press(() => useNavigation.getState().pop());
    expect(selectTop(useNavigation.getState()).kind).toBe('home');
  });

  it('Ocultar exige confirmacao com "Cancelar" em foco e so entao chama game_hide', async () => {
    const core = mockCore({ game_hide: null });
    await openOptions(game());
    // Sem layout no jsdom a navegacao espacial nao tem geometria: foca o item direto.
    press(() => setFocus('menu-hide'));
    await waitFor(() => expect(focusedLabel()).toContain('Ocultar'));
    press(activateFocused);
    await screen.findByRole('alertdialog');
    await waitFor(() => expect(screen.getByRole('button', { name: 'Cancelar' })).toHaveAttribute('data-focused', 'true'));
    expect(core.count('game_hide')).toBe(0);
    press(() => setFocus('dlg-confirm'));
    await waitFor(() => expect(screen.getByRole('button', { name: 'Ocultar' })).toHaveAttribute('data-focused', 'true'));
    press(activateFocused);
    await waitFor(() => expect(core.count('game_hide')).toBe(1));
    await waitFor(() => expect(useNavigation.getState().stack).toHaveLength(1));
    expect(useLibrary.getState().notice).toContain('oculto');
  });
});

describe('renomear (CA-61)', () => {
  async function openRename() {
    await openOptions(game());
    press(() => setFocus('menu-rename'));
    await waitFor(() => expect(focusedLabel()).toContain('Renomear'));
    press(activateFocused);
    return screen.findByLabelText('Novo nome');
  }
  const type = (input: HTMLElement, value: string) =>
    act(() => {
      const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value')!.set!;
      setter.call(input, value);
      input.dispatchEvent(new Event('input', { bubbles: true }));
    });

  it('comeca com o nome atual; Enter salva com o nome novo', async () => {
    const core = mockCore({ game_rename: game({ name: 'Novo nome' }) });
    const input = await openRename();
    expect(input).toHaveValue('Vale dos Ventos');
    type(input, '  Novo nome  ');
    press(activateFocused);
    await waitFor(() => expect(core.count('game_rename')).toBe(1));
    expect(core.calls[0]!.args).toEqual({ gameId: 'steam:1', name: 'Novo nome' });
    await waitFor(() => expect(useNavigation.getState().stack).toHaveLength(1));
    expect(useLibrary.getState().notice).toBe('Nome salvo.');
  });

  it('nome vazio e recusado, sem chamar o nucleo', async () => {
    const core = mockCore({ game_rename: game() });
    const input = await openRename();
    type(input, '   ');
    press(activateFocused);
    expect(await screen.findByRole('alert')).toHaveTextContent('O nome não pode ficar vazio.');
    expect(core.count('game_rename')).toBe(0);
  });

  it('EmptyName vindo do nucleo tambem aparece como erro', async () => {
    mockCore({ game_rename: rejectWith({ code: 'EmptyName' }) });
    const input = await openRename();
    type(input, 'x');
    press(activateFocused);
    expect(await screen.findByRole('alert')).toHaveTextContent('O nome não pode ficar vazio.');
  });

  it('Esc e B cancelam sem alterar nada', async () => {
    const core = mockCore({ game_rename: game() });
    const input = await openRename();
    type(input, 'Outro nome');
    press(runBackHandler);
    expect(screen.queryByLabelText('Novo nome')).toBeNull();
    expect(core.count('game_rename')).toBe(0);
    expect(selectTop(useNavigation.getState()).kind).toBe('gameOptions');
  });
});
