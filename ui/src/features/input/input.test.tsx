import { act, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { resetSpatialNavigation } from '@/app/navigation/spatial';
import { HomeScreen } from '@/features/library/HomeScreen';
import { useLibrary } from '@/features/library/store';
import { emitCoreEvent, mockCore, resetCore } from '@/shared/ipc/testing';
import { ControllerBanner, ControllerChip } from './ControllerStatus';
import { CONTROLLER_OK_MS, startControllers, useControllers } from './store';

const NONE = 'Nenhum controle conectado. Ligue o controle ou conecte o cabo.';
const LOST = 'Controle desconectado. Ligue o controle ou conecte o cabo.';

beforeEach(() => {
  resetSpatialNavigation();
  useControllers.setState({ connected: null, banner: 'none' });
});
afterEach(() => {
  resetCore();
  vi.useRealTimers();
});

describe('controle ausente (CA-13 a CA-15)', () => {
  it('abre sem controle: faixa de aviso e chip "Sem controle"', async () => {
    mockCore({ input_status: { connected: 0 } });
    const stop = startControllers();
    render(
      <>
        <ControllerBanner />
        <ControllerChip />
      </>,
    );
    expect(await screen.findByText(NONE)).toBeInTheDocument();
    expect(screen.getByText('Sem controle')).toBeInTheDocument();
    stop();
  });

  it('com controle ao abrir: sem faixa', async () => {
    mockCore({ input_status: { connected: 1 } });
    const stop = startControllers();
    render(
      <>
        <ControllerBanner />
        <ControllerChip />
      </>,
    );
    expect(await screen.findByText('Controle conectado')).toBeInTheDocument();
    expect(screen.queryByRole('status')).toBeNull();
    stop();
  });

  it('desconecta: aviso; volta: "Controle conectado." e some sozinho', async () => {
    mockCore({ input_status: { connected: 1 } });
    const stop = startControllers();
    render(<ControllerBanner />);
    await waitFor(() => expect(useControllers.getState().connected).toBe(1));
    await emitCoreEvent('input://controllers', { connected: 0 });
    expect(await screen.findByText(LOST)).toBeInTheDocument();
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    await act(async () => {
      await emitCoreEvent('input://controllers', { connected: 1 });
    });
    expect(screen.getByText('Controle conectado.')).toBeInTheDocument();
    act(() => void vi.advanceTimersByTime(CONTROLLER_OK_MS + 10));
    expect(screen.queryByText('Controle conectado.')).toBeNull();
    stop();
  });

  it('o aviso nao tira nem muda o foco da Home', async () => {
    mockCore({ input_status: { connected: 1 } });
    useLibrary.setState({
      status: 'ready',
      games: [
        { id: 'steam:1', name: 'Alfa', originalName: null, origin: 'Steam', cover: 'cover://steam:1', lastPlayedAt: 5, inProgress: false },
      ],
    });
    const stop = startControllers();
    render(
      <>
        <HomeScreen active />
        <ControllerBanner />
      </>,
    );
    const focusedKey = () => document.querySelector<HTMLElement>('[data-focused="true"]')?.dataset.focusKey;
    await waitFor(() => expect(focusedKey()).toBe('r:steam:1'));
    await emitCoreEvent('input://controllers', { connected: 0 });
    await screen.findByText(LOST);
    expect(document.querySelectorAll('[data-focused="true"]')).toHaveLength(1);
    expect(focusedKey()).toBe('r:steam:1');
    stop();
  });
});
