import { act, cleanup, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { ScreenStack } from '@/app/ScreenStack';
import { defaultDeps, dispatchIntent } from '@/app/navigation/dispatch';
import { resetSpatialNavigation } from '@/app/navigation/spatial';
import { useNavigation } from '@/app/navigation/store';
import { useLibrary } from '@/features/library/store';
import { IDLE_SESSION } from '@/features/session/overlay';
import { useSession } from '@/features/session/store';
import type { InputIntent } from '@/shared/input/intents';
import type { UpdateDto } from '@/shared/ipc';
import { emitCoreEvent, mockCore, rejectWith, resetCore } from '@/shared/ipc/testing';
import { progressLabel, shortNotes } from './domain';
import { startUpdates, useUpdate } from './store';

const UPDATE: UpdateDto = { version: '0.3.0', currentVersion: '0.2.0', notes: '## Novidades\n- Atualiza pelo app\n- **Correções**' };

const intent = (i: InputIntent) => act(() => void dispatchIntent({ intent: i, repeat: false }, defaultDeps()));
const updateDialogs = () =>
  useNavigation.getState().stack.filter((e) => e.kind === 'confirmDialog' && e.params?.confirm === 'update');
const flush = () => act(() => new Promise((r) => setTimeout(r, 0)));

let stops: Array<() => void> = [];
const start = () => stops.push(startUpdates(0));

beforeEach(() => {
  resetSpatialNavigation();
  useNavigation.getState().reset();
  useSession.setState({ session: IDLE_SESSION, failure: null, replace: null });
  useLibrary.setState({ status: 'ready', games: [], notice: null, refreshing: false });
  useUpdate.setState({ available: null, phase: 'idle', progress: null, offered: false });
});
afterEach(() => {
  stops.forEach((s) => s());
  stops = [];
  cleanup();
  resetCore();
});

describe('regras puras da atualizacao', () => {
  it('progresso com e sem total', () => {
    expect(progressLabel(null)).toBe('Preparando o download…');
    expect(progressLabel({ downloaded: 5 * 1024 * 1024, total: 10 * 1024 * 1024 })).toBe('50% · 5,0 MB de 10,0 MB');
    expect(progressLabel({ downloaded: 1024 * 1024, total: null })).toBe('1,0 MB baixados');
  });

  it('notas sem marcacao, curtas e como texto', () => {
    expect(shortNotes(UPDATE)).toEqual(['Novidades', 'Atualiza pelo app', 'Correções']);
    expect(shortNotes({ ...UPDATE, notes: null })).toEqual([]);
    const long = shortNotes({ ...UPDATE, notes: Array.from({ length: 9 }, (_, i) => `linha ${i}`).join('\n') });
    expect(long).toHaveLength(4);
    expect(long[3]).toMatch(/…$/);
  });
});

describe('procura automatica ao abrir', () => {
  it('versao nova sem jogo aberto: abre o dialogo uma vez', async () => {
    const core = mockCore({ update_check: UPDATE });
    start();
    await waitFor(() => expect(updateDialogs()).toHaveLength(1));
    expect(core.count('update_check')).toBe(1);
    expect(useUpdate.getState().offered).toBe(true);
  });

  it('ja na versao mais nova: nada aparece', async () => {
    const core = mockCore({ update_check: null });
    start();
    await waitFor(() => expect(core.count('update_check')).toBe(1));
    await flush();
    expect(updateDialogs()).toHaveLength(0);
    expect(useLibrary.getState().notice).toBeNull();
  });

  it('sem internet: falha em silencio', async () => {
    const core = mockCore({ update_check: rejectWith({ code: 'UpdateFailed' }) });
    start();
    await waitFor(() => expect(core.count('update_check')).toBe(1));
    await flush();
    expect(updateDialogs()).toHaveLength(0);
    expect(useLibrary.getState().notice).toBeNull();
  });

  it('com jogo aberto espera a sessao voltar a Idle', async () => {
    useSession.setState({ session: { ...IDLE_SESSION, state: 'Running', gameId: 'steam:620' } });
    const core = mockCore({ update_check: UPDATE });
    start();
    await waitFor(() => expect(core.count('update_check')).toBe(1));
    await flush();
    expect(updateDialogs()).toHaveLength(0);
    act(() => useSession.setState({ session: IDLE_SESSION }));
    expect(updateDialogs()).toHaveLength(1);
  });
});

describe('procura manual (Configuracoes)', () => {
  it('ja na mais nova avisa; falha mostra o texto do erro', async () => {
    mockCore({ update_check: null });
    await act(() => useUpdate.getState().check(true));
    expect(useLibrary.getState().notice).toBe('Você já está na versão mais nova.');
    mockCore({ update_check: rejectWith({ code: 'UpdateFailed' }) });
    await act(() => useUpdate.getState().check(true));
    expect(useLibrary.getState().notice).toMatch(/Não foi possível baixar a atualização/);
  });

  it('versao nova abre o dialogo', async () => {
    mockCore({ update_check: UPDATE });
    await act(() => useUpdate.getState().check(true));
    expect(updateDialogs()).toHaveLength(1);
  });
});

describe('dialogo de atualizacao', () => {
  async function openDialog() {
    render(<ScreenStack />);
    await waitFor(() => expect(document.querySelectorAll('[data-focused="true"]')).toHaveLength(1));
    act(() => {
      useUpdate.setState({ available: UPDATE, offered: true });
      useNavigation.getState().push('confirmDialog', undefined, { confirm: 'update' });
    });
    await screen.findByRole('alertdialog', { name: 'Nova versão disponível' });
  }

  it('mostra versoes, notas e o aviso do UAC; "Atualizar" em foco', async () => {
    mockCore();
    await openDialog();
    expect(screen.getByText(/SofaLauncher 0\.3\.0 está pronto para instalar/)).toBeInTheDocument();
    expect(screen.getByText(/versão 0\.2\.0/)).toBeInTheDocument();
    expect(screen.getByText('Atualiza pelo app')).toBeInTheDocument();
    expect(screen.getByText(/Confirme com teclado ou mouse/)).toBeInTheDocument();
    await waitFor(() => expect(document.querySelector('[data-focused="true"]')?.textContent).toContain('Atualizar'));
  });

  it('B ("Agora não") fecha sem baixar', async () => {
    const core = mockCore();
    await openDialog();
    intent('Back');
    await waitFor(() => expect(updateDialogs()).toHaveLength(0));
    expect(core.count('update_install')).toBe(0);
  });

  it('A baixa com progresso; UAC recusado volta com aviso e fecha o dialogo', async () => {
    let fail: (reason: unknown) => void = () => undefined;
    const core = mockCore({ update_install: () => new Promise((_, reject) => (fail = reject)) });
    start();
    await openDialog();
    await waitFor(() => expect(document.querySelector('[data-focused="true"]')?.textContent).toContain('Atualizar'));
    intent('Confirm');
    await screen.findByText('Baixando a atualização…');
    expect(core.count('update_install')).toBe(1);
    await act(() => emitCoreEvent('update://progress', { downloaded: 512 * 1024, total: 1024 * 1024 }));
    expect(await screen.findByText(/50% · 0,5 MB de 1,0 MB/)).toBeInTheDocument();
    await act(() => emitCoreEvent('update://progress', { downloaded: 1024 * 1024, total: 1024 * 1024 }));
    expect(await screen.findByText('Abrindo o instalador…')).toBeInTheDocument();
    await act(async () => fail({ code: 'UpdateCancelled' }));
    await waitFor(() => expect(updateDialogs()).toHaveLength(0));
    expect(useLibrary.getState().notice).toMatch(/o Windows não deu permissão/);
    expect(useUpdate.getState().phase).toBe('idle');
  });

  it('jogo aberto no meio do caminho: pede para fechar o jogo', async () => {
    mockCore({ update_install: rejectWith({ code: 'InvalidState' }) });
    await act(() => useUpdate.getState().install());
    expect(useLibrary.getState().notice).toBe('Feche o jogo antes de atualizar o SofaLauncher.');
  });
});
