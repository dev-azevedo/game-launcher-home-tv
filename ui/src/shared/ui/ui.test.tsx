import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { activateFocused, runBackHandler } from '@/shared/input/activation';
import { startSpatialNavigation } from '@/app/navigation/spatial';
import { InProgressBadge, OriginBadge } from './Badges';
import { Banner } from './Banner';
import { ConfirmDialog } from './ConfirmDialog';
import { CoverImage } from './Cover';
import { MenuList } from './MenuList';
import { WaitScreen } from './WaitScreen';

startSpatialNavigation();

const focused = () => document.querySelectorAll('[data-focused="true"]');

describe('ConfirmDialog', () => {
  it('abre com "Cancelar" em foco quando defaultFocus=cancel (RN-14, CA-43)', async () => {
    render(
      <ConfirmDialog
        title="Desligar o PC?"
        defaultFocus="cancel"
        confirmLabel="Desligar"
        destructive
        onConfirm={vi.fn()}
        onCancel={vi.fn()}
      />,
    );
    await waitFor(() => expect(screen.getByRole('button', { name: 'Cancelar' })).toHaveAttribute('data-focused', 'true'));
    expect(focused()).toHaveLength(1);
    expect(screen.getByRole('alertdialog')).toHaveAttribute('aria-modal', 'true');
  });

  it('A no item em foco cancela; B tambem; confirmar so quando focado', async () => {
    const onCancel = vi.fn();
    const onConfirm = vi.fn();
    render(
      <ConfirmDialog title="Ocultar?" defaultFocus="cancel" confirmLabel="Ocultar" onConfirm={onConfirm} onCancel={onCancel} />,
    );
    await waitFor(() => expect(focused()).toHaveLength(1));
    act(() => void activateFocused());
    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(onConfirm).not.toHaveBeenCalled();
    act(() => void runBackHandler());
    expect(onCancel).toHaveBeenCalledTimes(2);
  });

  it('defaultFocus=confirm foca o botao de confirmar', async () => {
    render(
      <ConfirmDialog title="Restaurar?" defaultFocus="confirm" confirmLabel="Restaurar" onConfirm={vi.fn()} onCancel={vi.fn()} />,
    );
    await waitFor(() => expect(screen.getByRole('button', { name: 'Restaurar' })).toHaveAttribute('data-focused', 'true'));
  });

  it('nome com HTML aparece literal, sem criar elemento (SEC-07)', () => {
    const evil = '<img src=x onerror=alert(1)>';
    const { container } = render(
      <ConfirmDialog title={`Ocultar ${evil}?`} warning={`${evil} esta aberto.`} defaultFocus="cancel" confirmLabel="Ocultar" onConfirm={vi.fn()} onCancel={vi.fn()}>
        {`Jogo ${evil}`}
      </ConfirmDialog>,
    );
    expect(screen.getByRole('heading')).toHaveTextContent(`Ocultar ${evil}?`);
    expect(container.querySelector('img')).toBeNull();
  });
});

describe('WaitScreen (RN-31, CA-77, CA-78)', () => {
  beforeEach(() => vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] }));
  afterEach(() => vi.useRealTimers());

  it('ignora A antes de 600 ms e aceita depois; B funciona desde o inicio', async () => {
    const onLeave = vi.fn();
    render(
      <WaitScreen title="Abrindo Minério…" buttonLabel="Voltar" onLeave={onLeave}>
        Aguarde.
      </WaitScreen>,
    );
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(screen.getByRole('button', { name: 'Voltar' })).toHaveAttribute('data-focused', 'true');

    act(() => void activateFocused());
    expect(onLeave).not.toHaveBeenCalled();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(599);
    });
    act(() => void activateFocused());
    expect(onLeave).not.toHaveBeenCalled();

    await act(async () => {
      await vi.advanceTimersByTimeAsync(1);
    });
    act(() => void activateFocused());
    expect(onLeave).toHaveBeenCalledTimes(1);
  });

  it('B sai imediatamente', async () => {
    const onLeave = vi.fn();
    render(<WaitScreen title="Fechando…" buttonLabel="Ir para a biblioteca" onLeave={onLeave} />);
    act(() => void runBackHandler());
    expect(onLeave).toHaveBeenCalledTimes(1);
  });
});

describe('CoverImage (CA-24, SEC-08)', () => {
  it('usa carga preguicosa, decodificacao assincrona e dimensoes fixas', () => {
    const { container } = render(<CoverImage src="cover://steam:1" name="Minério" />);
    const img = container.querySelector('img')!;
    expect(img).toHaveAttribute('loading', 'lazy');
    expect(img).toHaveAttribute('decoding', 'async');
    expect(img).toHaveAttribute('width', '600');
    expect(img).toHaveAttribute('height', '900');
    expect(img).toHaveAttribute('alt', '');
  });

  it('capa com 404 vira placeholder com o nome', () => {
    const { container } = render(<CoverImage src="cover://steam:404" name="Minério" />);
    fireEvent.error(container.querySelector('img')!);
    expect(container.querySelector('img')).toBeNull();
    expect(screen.getByTestId('cover-placeholder')).toHaveTextContent('Minério');
  });

  it('sem capa mostra o placeholder; nome com HTML fica literal', () => {
    const evil = '<img src=x onerror=alert(1)>';
    const { container } = render(<CoverImage name={evil} />);
    expect(screen.getByTestId('cover-placeholder')).toHaveTextContent(evil);
    expect(container.querySelector('img')).toBeNull();
  });
});

describe('selos, aviso e menu', () => {
  it('origem e andamento', () => {
    render(
      <>
        <OriginBadge origin="Gog" />
        <OriginBadge origin="Manual" />
        <InProgressBadge />
      </>,
    );
    expect(screen.getByText('GOG')).toBeInTheDocument();
    expect(screen.getByText('Manual')).toBeInTheDocument();
    expect(screen.getByText('Em andamento')).toBeInTheDocument();
  });

  it('Banner e uma regiao de status e nao tem item focavel', () => {
    render(<Banner tone="warn">Nenhum controle conectado. Ligue o controle ou conecte o cabo.</Banner>);
    expect(screen.getByRole('status')).toHaveTextContent('Nenhum controle conectado.');
    expect(focused()).toHaveLength(0);
  });

  it('MenuList: item desabilitado fica aria-disabled e nao recebe foco', async () => {
    render(
      <MenuList
        label="Menu"
        items={[
          { id: 'a', label: 'Suspender', hint: 'Suspensão indisponível neste PC', disabled: true },
          { id: 'b', label: 'Configurações' },
        ]}
      />,
    );
    expect(screen.getByRole('menuitem', { name: /Suspender/ })).toHaveAttribute('aria-disabled', 'true');
    expect(screen.getByRole('menuitem', { name: /Suspender/ })).toHaveAttribute('data-focused', 'false');
  });
});
