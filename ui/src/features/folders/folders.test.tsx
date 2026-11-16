import { act, cleanup, render, screen, waitFor } from '@testing-library/react';
import { getCurrentFocusKey, setFocus } from '@noriginmedia/norigin-spatial-navigation';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { ScreenStack } from '@/app/ScreenStack';
import { defaultDeps, dispatchIntent } from '@/app/navigation/dispatch';
import { resetSpatialNavigation } from '@/app/navigation/spatial';
import { useNavigation } from '@/app/navigation/store';
import { useLibrary } from '@/features/library/store';
import { IDLE_SESSION } from '@/features/session/overlay';
import { useSession } from '@/features/session/store';
import { neutralizeName } from '@/shared/domain/text';
import type { InputIntent } from '@/shared/input/intents';
import type { DirEntry, DirListing, GameDto } from '@/shared/ipc';
import { mockCore, rejectWith, resetCore } from '@/shared/ipc/testing';
import { entryPath, gameNameFromExe, lastSegment, parentTarget } from './domain';

const intent = (i: InputIntent) => act(() => void dispatchIntent({ intent: i, repeat: false }, defaultDeps()));
const focusedText = () =>
  Array.from(document.querySelectorAll<HTMLElement>('[data-focused="true"]'))
    .map((e) => e.textContent)
    .join('|');
/** Move o foco e espera ele assentar (o foco inicial da lista pode chegar depois). */
async function focus(key: string) {
  // O foco inicial da lista (efeito) pode chegar depois do primeiro render: espera ele antes de mover.
  if (key.startsWith('fb-')) await waitFor(() => expect(getCurrentFocusKey()).toMatch(/^fb-/));
  act(() => void setFocus(key));
  await waitFor(() => expect(getCurrentFocusKey()).toBe(key));
}
const kinds = () => useNavigation.getState().stack.map((e) => e.kind);

const D = (name: string, kind: DirEntry['kind'] = 'Dir'): DirEntry => ({ name, kind });
const DRIVES: DirListing = { path: null, parent: null, entries: [D('C:\\', 'Drive'), D('D:\\', 'Drive')], truncated: false };
const C_ROOT: DirListing = { path: 'C:\\', parent: null, entries: [D('Jogos'), D('Users')], truncated: false };
const JOGOS: DirListing = {
  path: 'C:\\Jogos',
  parent: 'C:\\',
  entries: [D('Bruma.exe', 'Exe'), D('leiame.exe', 'Exe'), D('Aventura.exe', 'Exe')],
  truncated: false,
};
const FS: Record<string, DirListing> = { 'C:\\': C_ROOT, 'C:\\Jogos': JOGOS };
const listing = (args: Record<string, unknown>) => {
  const path = args.path as string | null;
  if (path === null) return DRIVES;
  const found = FS[path];
  return found ?? Promise.reject({ code: 'NotFound' });
};

const added: GameDto = {
  id: 'manual:9f3a1c2b7d4e5a60',
  name: 'Aventura',
  originalName: null,
  origin: 'Manual',
  cover: '',
  lastPlayedAt: null,
  inProgress: false,
};

async function openBrowser() {
  render(<ScreenStack />);
  await waitFor(() => expect(document.querySelectorAll('[data-focused="true"]')).toHaveLength(1));
  act(() => useNavigation.getState().push('folderBrowser'));
  await screen.findByText('Este computador');
  await screen.findByText('C:\\');
}

beforeEach(() => {
  resetSpatialNavigation();
  useNavigation.getState().reset();
  useSession.setState({ session: IDLE_SESSION, failure: null, replace: null });
  useLibrary.setState({ status: 'ready', games: [added], notice: null, refreshing: false });
});
afterEach(() => {
  cleanup();
  resetCore();
});

describe('regras puras do navegador (CT-03)', () => {
  it('neutraliza controle e bidi para exibicao, sem alterar texto comum', () => {
    expect(neutralizeName('Jogo normal ação.exe')).toBe('Jogo normal ação.exe');
    expect(neutralizeName('na\u202Eme\u0007.exe')).toBe('na\uFFFDme\u2407.exe');
    expect(neutralizeName('a\u0085b\u007Fc')).toBe('a\uFFFDb\u2421c');
    expect(neutralizeName('<img src=x onerror=alert(1)>')).toBe('<img src=x onerror=alert(1)>');
  });

  it('caminho da entrada, nome sugerido, pasta de cima e ultimo trecho', () => {
    expect(entryPath(null, D('C:\\', 'Drive'))).toBe('C:\\');
    expect(entryPath('C:\\', D('Jogos'))).toBe('C:\\Jogos');
    expect(entryPath('C:\\Jogos', D('Bruma.exe', 'Exe'))).toBe('C:\\Jogos\\Bruma.exe');
    expect(gameNameFromExe('Aventura.EXE')).toBe('Aventura');
    expect(parentTarget(null, null)).toBeUndefined();
    expect(parentTarget('C:\\', null)).toBeNull();
    expect(parentTarget('C:\\Jogos', 'C:\\')).toBe('C:\\');
    expect(lastSegment('C:\\Jogos\\')).toBe('Jogos');
  });
});

describe('navegador de pastas e Adicionar jogo (FE-13)', () => {
  it('lista as unidades; A abre a pasta; B volta a pasta anterior e na raiz sai (CA-19)', async () => {
    const c = mockCore({ fs_list_dir: listing });
    await openBrowser();
    expect(c.calls.find((x) => x.command === 'fs_list_dir')!.args).toEqual({ path: null });
    await waitFor(() => expect(getCurrentFocusKey()).toBe('fb-0'));
    intent('Confirm');
    await screen.findByText('Jogos');
    expect(c.calls.filter((x) => x.command === 'fs_list_dir').at(-1)!.args).toEqual({ path: 'C:\\' });
    intent('Back');
    await screen.findByText('D:\\');
    // O foco volta a unidade de onde saiu.
    await waitFor(() => expect(getCurrentFocusKey()).toBe('fb-0'));
    intent('Back');
    await waitFor(() => expect(kinds()).toEqual(['home']));
  });

  it('escolher um .exe abre a confirmacao com "Adicionar" em foco e o caminho; confirmar cadastra (CA-19)', async () => {
    const c = mockCore({ fs_list_dir: listing, library_add_manual: added });
    await openBrowser();
    await focus('fb-0');
    intent('Confirm');
    await screen.findByText('Jogos');
    await focus('fb-0');
    intent('Confirm');
    await screen.findByText('Bruma.exe');
    await focus('fb-2');
    intent('Confirm');
    expect(await screen.findByText('Adicionar este jogo?')).toBeInTheDocument();
    expect(screen.getByText('C:\\Jogos\\Aventura.exe')).toBeInTheDocument();
    expect(screen.getByText(/entra na biblioteca como Aventura/)).toBeInTheDocument();
    await waitFor(() => expect(getCurrentFocusKey()).toBe('dlg-confirm'));
    expect(c.count('library_add_manual')).toBe(0);
    intent('Confirm');
    await waitFor(() => expect(c.count('library_add_manual')).toBe(1));
    expect(c.calls.find((x) => x.command === 'library_add_manual')!.args).toEqual({ path: 'C:\\Jogos\\Aventura.exe' });
    await waitFor(() => expect(kinds()).toEqual(['home']));
    expect(useLibrary.getState().notice).toBe('Aventura adicionado.');
  });

  async function pickExe(index: number) {
    await openBrowser();
    await focus('fb-0');
    intent('Confirm');
    await screen.findByText('Jogos');
    await focus('fb-0');
    intent('Confirm');
    await screen.findByText('Bruma.exe');
    await focus(`fb-${index}`);
    intent('Confirm');
    await screen.findByText('Adicionar este jogo?');
  }

  it.each([
    ['Duplicate', 'Este jogo já está na biblioteca.'],
    ['NotAnExecutable', 'Este arquivo não é um executável válido.'],
    ['PathNotAllowed', 'Este local não é aceito. O SofaLauncher só abre jogos de discos deste PC.'],
  ])('erro %s no cadastro: mensagem propria e Voltar (CA-20, CA-71, CA-72)', async (code, message) => {
    mockCore({ fs_list_dir: listing, library_add_manual: rejectWith({ code }) });
    await pickExe(0);
    intent('Confirm');
    expect(await screen.findByText(message)).toBeInTheDocument();
    await waitFor(() => expect(getCurrentFocusKey()).toBe('dlg-cancel'));
    intent('Confirm');
    // Volta a lista, sem cadastrar nada.
    await waitFor(() => expect(screen.queryByText(message)).toBeNull());
    expect(kinds()).toEqual(['home', 'folderBrowser']);
  });

  it('Voltar na confirmacao nao cadastra', async () => {
    const c = mockCore({ fs_list_dir: listing, library_add_manual: added });
    await pickExe(0);
    intent('Back');
    await waitFor(() => expect(screen.queryByText('Adicionar este jogo?')).toBeNull());
    expect(c.count('library_add_manual')).toBe(0);
  });

  it('pasta sem permissao: aviso com Voltar em foco; Voltar mostra a lista de novo (CA-71)', async () => {
    mockCore({
      fs_list_dir: (args: Record<string, unknown>) =>
        args.path === 'C:\\' ? rejectWith({ code: 'AccessDenied' })({}) : listing(args),
    });
    await openBrowser();
    intent('Confirm');
    expect(await screen.findByText('Sem permissão para abrir esta pasta.')).toBeInTheDocument();
    expect(screen.getByText('O Windows não deixa o SofaLauncher ver o que há aqui.')).toBeInTheDocument();
    await waitFor(() => expect(focusedText()).toBe('Voltar'));
    intent('Confirm');
    await screen.findByText('C:\\');
    expect(screen.queryByText('Sem permissão para abrir esta pasta.')).toBeNull();
  });

  it('local fora dos discos (PathNotAllowed) e pasta que sumiu (NotFound) tem avisos distintos', async () => {
    mockCore({
      fs_list_dir: (args: Record<string, unknown>) =>
        args.path === 'C:\\' ? rejectWith({ code: 'PathNotAllowed' })({}) : listing(args),
    });
    await openBrowser();
    intent('Confirm');
    expect(await screen.findByText(/Este local não é aceito/)).toBeInTheDocument();
  });

  it('pasta vazia: mensagem e Voltar em foco', async () => {
    mockCore({
      fs_list_dir: (args: Record<string, unknown>) =>
        args.path === 'C:\\' ? { path: 'C:\\', parent: null, entries: [], truncated: false } : listing(args),
    });
    await openBrowser();
    intent('Confirm');
    expect(await screen.findByText('Nenhuma pasta ou .exe aqui')).toBeInTheDocument();
    await waitFor(() => expect(focusedText()).toBe('Voltar'));
  });

  it('lista cortada (truncated) avisa o usuario (CT-04)', async () => {
    mockCore({ fs_list_dir: () => ({ path: 'C:\\', parent: null, entries: [D('A')], truncated: true }) });
    render(<ScreenStack />);
    act(() => useNavigation.getState().push('folderBrowser'));
    expect(await screen.findByText(/A lista foi cortada/)).toBeInTheDocument();
  });

  it('nomes crus: HTML aparece literal e controle/bidi viram simbolo visivel, mas a navegacao usa o nome original', async () => {
    const evil = '<img src=x onerror=alert(1)>';
    const tricky = 'na\u202Eme\u0007';
    const c = mockCore({
      fs_list_dir: (args: Record<string, unknown>) =>
        args.path === null
          ? { path: 'D:\\', parent: null, entries: [D(evil), D(tricky)], truncated: false }
          : { path: String(args.path), parent: 'D:\\', entries: [], truncated: false },
    });
    render(<ScreenStack />);
    act(() => useNavigation.getState().push('folderBrowser'));
    expect(await screen.findByText(evil)).toBeInTheDocument();
    expect(document.querySelectorAll('img[onerror]')).toHaveLength(0);
    expect(screen.getByText('na\uFFFDme\u2407')).toBeInTheDocument();
    await focus('fb-1');
    intent('Confirm');
    await waitFor(() => expect(c.calls.filter((x) => x.command === 'fs_list_dir')).toHaveLength(2));
    expect(c.calls.at(-1)!.args).toEqual({ path: `D:\\${tricky}` });
  });

  it('lista longa (2.000 entradas) monta so as linhas perto da tela e o foco chega ao fim', async () => {
    const entries = Array.from({ length: 2000 }, (_, i) => D(`Arquivo ${String(i).padStart(4, '0')}.exe`, 'Exe'));
    mockCore({ fs_list_dir: () => ({ path: 'D:\\Truncada', parent: 'D:\\', entries, truncated: true }) });
    render(<ScreenStack />);
    act(() => useNavigation.getState().push('folderBrowser'));
    await screen.findByText('Arquivo 0000.exe');
    expect(document.querySelectorAll('.fb-row').length).toBeLessThan(60);
    expect(screen.getByText(/A lista foi cortada/)).toHaveTextContent('2.000');
  });
});
