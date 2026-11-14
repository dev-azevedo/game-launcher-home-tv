import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { act, cleanup, render, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { ScreenStack } from '@/app/ScreenStack';
import { resetSpatialNavigation } from '@/app/navigation/spatial';
import { useNavigation } from '@/app/navigation/store';
import { useLibrary } from '@/features/library/store';
import { IDLE_SESSION } from '@/features/session/overlay';
import { useSession } from '@/features/session/store';
import { emitCoreEvent, mockCore, rejectWith, resetCore } from '@/shared/ipc/testing';
import { reloadAfterResume, startBackgroundGuard, startSystemResume } from './system';

const GAME = { id: 'steam:1', name: 'Alfa', originalName: null, origin: 'Steam', cover: '', lastPlayedAt: 1, inProgress: false } as const;
const kinds = () => useNavigation.getState().stack.map((e) => e.kind);

let stop: () => void = () => undefined;
beforeEach(() => {
  resetSpatialNavigation();
  useNavigation.getState().reset();
  useSession.setState({ session: IDLE_SESSION, failure: null, replace: null });
  useLibrary.setState({ status: 'ready', games: [GAME], notice: null });
});
afterEach(() => {
  stop();
  cleanup();
  resetCore();
});

describe('volta da suspensao (FE-14, CA-46)', () => {
  it('a pilha de telas continua igual e o estado e recarregado', async () => {
    const core = mockCore({
      library_get: { games: [GAME, { ...GAME, id: 'steam:2', name: 'Beta' }], focusGameId: null },
      session_get: IDLE_SESSION,
      volume_get: { level: 30, muted: false, available: true },
      input_status: { connected: 1 },
      power_capabilities: { shutdown: 'Available', restart: 'Available', suspend: 'Available' },
    });
    stop = startSystemResume();
    render(<ScreenStack />);
    act(() => {
      useNavigation.getState().push('quickMenu');
      useNavigation.getState().push('confirmDialog', undefined, { confirm: 'exit' });
    });
    const before = kinds();
    expect(before).toEqual(['home', 'quickMenu', 'confirmDialog']);
    await emitCoreEvent('system://resumed', {});
    await waitFor(() => expect(core.count('library_get')).toBe(1));
    await waitFor(() => expect(useLibrary.getState().games).toHaveLength(2));
    expect(core.count('session_get')).toBe(1);
    expect(core.count('input_status')).toBe(1);
    expect(kinds()).toEqual(before);
  });

  it('falha ao recarregar nao esvazia a biblioteca na tela', async () => {
    mockCore({ library_get: rejectWith({ code: 'Internal' }) });
    reloadAfterResume();
    await new Promise((r) => setTimeout(r, 30));
    expect(useLibrary.getState().games).toHaveLength(1);
    expect(useLibrary.getState().status).toBe('ready');
  });
});

describe('segundo plano (FE-14, RN-21)', () => {
  const setVisibility = (state: 'hidden' | 'visible') => {
    Object.defineProperty(document, 'visibilityState', { configurable: true, get: () => state });
    document.dispatchEvent(new Event('visibilitychange'));
  };

  it('janela oculta marca o <html> e voltar a mostrar desmarca', () => {
    const off = startBackgroundGuard();
    expect(document.documentElement.dataset.windowHidden).toBe('false');
    setVisibility('hidden');
    expect(document.documentElement.dataset.windowHidden).toBe('true');
    setVisibility('visible');
    expect(document.documentElement.dataset.windowHidden).toBe('false');
    off();
    setVisibility('hidden');
    expect(document.documentElement.dataset.windowHidden).toBe('false');
    setVisibility('visible');
  });

  it('o CSS pausa todas as animacoes com a janela oculta (nenhuma roda em segundo plano)', () => {
    const css = readFileSync(resolve(process.cwd(), 'src/app/global.css'), 'utf8');
    expect(css).toMatch(/html\[data-window-hidden="true"\] \*[^{]*\{animation-play-state:paused!important\}/);
    expect(css).toContain('html[data-window-hidden="true"] *::before');
    expect(css).toContain('html[data-window-hidden="true"] *::after');
  });
});
