import { act, render, screen, waitFor } from '@testing-library/react';
import { getCurrentFocusKey, setFocus } from '@noriginmedia/norigin-spatial-navigation';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { ScreenStack } from '@/app/ScreenStack';
import { VolumeHud } from '@/features/volume/VolumeHud';
import { defaultDeps, dispatchIntent } from '@/app/navigation/dispatch';
import { resetSpatialNavigation } from '@/app/navigation/spatial';
import { selectTop, useNavigation } from '@/app/navigation/store';
import { useLibrary } from '@/features/library/store';
import { IDLE_SESSION } from '@/features/session/overlay';
import { useSession } from '@/features/session/store';
import { startVolume, useVolume } from '@/features/volume/store';
import { hudLabel } from '@/features/volume/domain';
import { activateFocused } from '@/shared/input/activation';
import type { InputIntent } from '@/shared/input/intents';
import { emitCoreEvent, mockCore, rejectWith, resetCore } from '@/shared/ipc/testing';
import { powerCopy } from './power';

const press = (fn: () => unknown) => act(() => void fn());
const intent = (i: InputIntent) => act(() => void dispatchIntent({ intent: i, repeat: false }, defaultDeps()));
// Texto do item com o foco do norigin (no jsdom o `data-focused` do item anterior pode demorar a apagar).
const focusedText = () =>
  Array.from(document.querySelectorAll<HTMLElement>('[data-focus-key]'))
    .filter((e) => e.dataset.focusKey === getCurrentFocusKey())
    .map((e) => e.textContent)
    .join('|');
const kinds = () => useNavigation.getState().stack.map((e) => e.kind);
const CAPS = { shutdown: 'Available', restart: 'Available', suspend: 'Available' };
const VOL = { level: 40, muted: false, available: true };

let stop: () => void = () => undefined;
async function renderHome() {
  const result = render(
    <>
      <ScreenStack />
      <VolumeHud />
    </>,
  );
  await waitFor(() => expect(document.querySelectorAll('[data-focused="true"]')).toHaveLength(1));
  return result;
}
async function openMenu() {
  intent('Menu');
  await screen.findByRole('dialog', { name: 'Menu rápido' });
}

beforeEach(() => {
  resetSpatialNavigation();
  useNavigation.getState().reset();
  useSession.setState({ session: IDLE_SESSION, failure: null, replace: null });
  useVolume.setState({ volume: null, hud: false });
  useLibrary.setState({
    status: 'ready',
    games: [{ id: 'steam:1', name: 'Minério', originalName: null, origin: 'Steam', cover: '', lastPlayedAt: 1, inProgress: false }],
    notice: null,
  });
});
afterEach(() => {
  stop();
  resetCore();
  vi.useRealTimers();
});

describe('textos de energia', () => {
  it('reiniciar avisa que o Sofa volta so com "Iniciar com o Windows" ligado', () => {
    expect(powerCopy('Restart', { autostart: true }).body).toContain('O Sofá abre de novo');
    expect(powerCopy('Restart', { autostart: false }).body).not.toContain('abre de novo');
    expect(hudLabel({ level: 45, muted: false, available: true })).toBe('45');
    expect(hudLabel({ level: 45, muted: true, available: true })).toBe('Mudo');
    expect(hudLabel({ level: 0, muted: false, available: false })).toBe('Sem áudio');
  });
});

describe('menu rapido (FE-10)', () => {
  it('Menu abre com Volume em foco; o nivel vem de volume://changed (CA-41); B fecha', async () => {
    mockCore({ volume_get: VOL, power_capabilities: CAPS });
    stop = startVolume();
    await renderHome();
    await waitFor(() => expect(useVolume.getState().volume?.level).toBe(40));
    await openMenu();
    await waitFor(() => expect(focusedText()).toContain('Volume'));
    expect(screen.getByRole('slider', { name: 'Volume' })).toHaveAttribute('aria-valuenow', '40');
    await emitCoreEvent('volume://changed', { level: 65, muted: false, available: true });
    await waitFor(() => expect(screen.getByRole('slider', { name: 'Volume' })).toHaveAttribute('aria-valuenow', '65'));
    intent('Back');
    await waitFor(() => expect(kinds()).toEqual(['home']));
  });

  it('←/→ no Volume chamam volume_step (+5/-5) e nao movem o foco; nos outros itens seguem o foco', async () => {
    const core = mockCore({ volume_get: VOL, power_capabilities: CAPS, volume_step: VOL });
    stop = startVolume();
    await renderHome();
    await openMenu();
    await waitFor(() => expect(focusedText()).toContain('Volume'));
    intent('Right');
    intent('Left');
    await waitFor(() => expect(core.count('volume_step')).toBe(2));
    expect(core.calls.filter((c) => c.command === 'volume_step').map((c) => c.args)).toEqual([
      { direction: 'Up' },
      { direction: 'Down' },
    ]);
    expect(focusedText()).toContain('Volume');
  });

  it('Mudo liga e desliga pelo nucleo (CA-39)', async () => {
    const core = mockCore({ volume_get: VOL, power_capabilities: CAPS, volume_toggle_mute: { ...VOL, muted: true } });
    stop = startVolume();
    await renderHome();
    await openMenu();
    press(() => setFocus('menu-mute'));
    await waitFor(() => expect(focusedText()).toContain('Mudo'));
    press(activateFocused);
    await waitFor(() => expect(core.count('volume_toggle_mute')).toBe(1));
    await waitFor(() => expect(screen.getByRole('switch', { name: /Mudo/ })).toHaveAttribute('aria-checked', 'true'));
  });

  it('sem dispositivo de audio: Volume e Mudo desabilitados com o motivo e o foco vai para Desligar (CA-42)', async () => {
    mockCore({ volume_get: { level: 0, muted: false, available: false }, power_capabilities: CAPS });
    stop = startVolume();
    await renderHome();
    await waitFor(() => expect(useVolume.getState().volume?.available).toBe(false));
    await openMenu();
    expect(screen.getByText('Nenhum dispositivo de áudio encontrado')).toBeInTheDocument();
    expect(screen.getByRole('slider', { name: 'Volume' })).toHaveAttribute('aria-disabled', 'true');
    await waitFor(() => expect(focusedText()).toContain('Desligar'));
  });

  it('suspensao indisponivel: desabilitada, com o motivo (CA-47)', async () => {
    mockCore({ volume_get: VOL, power_capabilities: { ...CAPS, suspend: 'Unavailable' } });
    stop = startVolume();
    await renderHome();
    await openMenu();
    expect(await screen.findByText('Suspensão indisponível neste PC')).toBeInTheDocument();
    expect(screen.getByRole('menuitem', { name: /Suspender/ })).toHaveAttribute('aria-disabled', 'true');
  });

  it('Desligar: power_prepare ao abrir, "Cancelar" em foco, power_execute so ao confirmar com o ticket (SEC-27)', async () => {
    const core = mockCore({ volume_get: VOL, power_capabilities: CAPS, power_prepare: 'TICKET-0123456789ABCDEFGH', power_execute: null });
    stop = startVolume();
    await renderHome();
    await openMenu();
    press(() => setFocus('menu-off'));
    await waitFor(() => expect(focusedText()).toContain('Desligar'));
    press(activateFocused);
    expect(await screen.findByText('Desligar o PC?')).toBeInTheDocument();
    await waitFor(() => expect(focusedText()).toBe('Cancelar'));
    expect(core.calls.find((c) => c.command === 'power_prepare')!.args).toEqual({ action: 'Shutdown' });
    expect(core.count('power_execute')).toBe(0);
    press(() => setFocus('dlg-confirm'));
    await waitFor(() => expect(focusedText()).toBe('Desligar'));
    press(activateFocused);
    await waitFor(() => expect(core.count('power_execute')).toBe(1));
    expect(core.calls.find((c) => c.command === 'power_execute')!.args).toEqual({ ticket: 'TICKET-0123456789ABCDEFGH' });
    expect(await screen.findByText('Desligando…')).toBeInTheDocument();
  });

  it('Cancelar e B nao executam nada e voltam ao menu', async () => {
    const core = mockCore({ volume_get: VOL, power_capabilities: CAPS, power_prepare: 'TICKET-0123456789ABCDEFGH', power_execute: null });
    stop = startVolume();
    await renderHome();
    await openMenu();
    press(() => setFocus('menu-restart'));
    await waitFor(() => expect(focusedText()).toContain('Reiniciar'));
    press(activateFocused);
    await screen.findByText('Reiniciar o PC?');
    intent('Back');
    await waitFor(() => expect(kinds()).toEqual(['home', 'quickMenu']));
    expect(core.count('power_execute')).toBe(0);
  });

  it('com jogo aberto a confirmacao avisa o nome (CA-48)', async () => {
    mockCore({ volume_get: VOL, power_capabilities: CAPS, power_prepare: 'TICKET-0123456789ABCDEFGH' });
    useSession.setState({ session: { ...IDLE_SESSION, state: 'Running', gameId: 'steam:1' } });
    stop = startVolume();
    await renderHome();
    await openMenu();
    press(() => setFocus('menu-off'));
    await waitFor(() => expect(focusedText()).toContain('Desligar'));
    press(activateFocused);
    expect(await screen.findByText('Minério está aberto. O progresso não salvo pode ser perdido.')).toBeInTheDocument();
  });

  it('ticket expirado: mensagem propria e Voltar', async () => {
    mockCore({
      volume_get: VOL,
      power_capabilities: CAPS,
      power_prepare: 'TICKET-0123456789ABCDEFGH',
      power_execute: rejectWith({ code: 'TicketInvalid' }),
    });
    stop = startVolume();
    await renderHome();
    await openMenu();
    press(() => setFocus('menu-off'));
    await waitFor(() => expect(focusedText()).toContain('Desligar'));
    press(activateFocused);
    await screen.findByText('Desligar o PC?');
    press(() => setFocus('dlg-confirm'));
    await waitFor(() => expect(focusedText()).toBe('Desligar'));
    press(activateFocused);
    expect(await screen.findByText('A confirmação expirou. Escolha a ação de novo.')).toBeInTheDocument();
    await waitFor(() => expect(focusedText()).toBe('Voltar'));
  });

  it('Sair do launcher: confirma com Cancelar em foco e chama app_exit; o jogo segue aberto', async () => {
    const core = mockCore({ volume_get: VOL, power_capabilities: CAPS, app_exit: null });
    useSession.setState({ session: { ...IDLE_SESSION, state: 'Running', gameId: 'steam:1' } });
    stop = startVolume();
    await renderHome();
    await openMenu();
    press(() => setFocus('menu-exit'));
    await waitFor(() => expect(focusedText()).toContain('Sair do launcher'));
    press(activateFocused);
    expect(await screen.findByText('Minério continuará aberto.')).toBeInTheDocument();
    await waitFor(() => expect(focusedText()).toBe('Cancelar'));
    expect(core.count('app_exit')).toBe(0);
    press(() => setFocus('dlg-confirm'));
    await waitFor(() => expect(focusedText()).toBe('Sair do Sofá'));
    press(activateFocused);
    await waitFor(() => expect(core.count('app_exit')).toBe(1));
  });

  it('Configuracoes fecha o menu e abre a tela de configuracoes', async () => {
    mockCore({ volume_get: VOL, power_capabilities: CAPS });
    stop = startVolume();
    await renderHome();
    await openMenu();
    press(() => setFocus('menu-settings'));
    await waitFor(() => expect(focusedText()).toContain('Configurações'));
    press(activateFocused);
    await waitFor(() => expect(kinds()).toEqual(['home', 'settings']));
  });
});

describe('VolumeHud e LT/RT (FE-11)', () => {
  it('RT na Home chama volume_step Up e mostra o indicador; LT chama Down', async () => {
    const core = mockCore({ volume_get: VOL, volume_step: { level: 45, muted: false, available: true } });
    stop = startVolume();
    await renderHome();
    intent('VolumeUp');
    expect(await screen.findByRole('status')).toHaveTextContent('45');
    intent('VolumeDown');
    await waitFor(() => expect(core.count('volume_step')).toBe(2));
    expect(core.calls.filter((c) => c.command === 'volume_step').map((c) => c.args.direction)).toEqual(['Up', 'Down']);
  });

  it('o indicador some 2 s depois do ultimo uso', async () => {
    mockCore({ volume_get: VOL, volume_step: VOL });
    stop = startVolume();
    await renderHome();
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    await act(async () => void (await useVolume.getState().step('Up', { hud: true })));
    expect(useVolume.getState().hud).toBe(true);
    act(() => void vi.advanceTimersByTime(1900));
    expect(useVolume.getState().hud).toBe(true);
    act(() => void vi.advanceTimersByTime(200));
    expect(useVolume.getState().hud).toBe(false);
  });

  it('sem dispositivo, LT/RT mostram "Sem áudio"', async () => {
    mockCore({ volume_get: VOL, volume_step: rejectWith({ code: 'NoAudioDevice' }) });
    stop = startVolume();
    await renderHome();
    intent('VolumeUp');
    expect(await screen.findByText('Sem áudio')).toBeInTheDocument();
  });

  it('LT/RT sao ignorados no menu rapido, em dialogos e no painel (CA-65)', async () => {
    const core = mockCore({ volume_get: VOL, power_capabilities: CAPS, volume_step: VOL, power_prepare: 'TICKET-0123456789ABCDEFGH' });
    stop = startVolume();
    await renderHome();
    await openMenu();
    intent('VolumeUp');
    press(() => setFocus('menu-off'));
    await waitFor(() => expect(focusedText()).toContain('Desligar'));
    press(activateFocused);
    await screen.findByText('Desligar o PC?');
    intent('VolumeDown');
    expect(selectTop(useNavigation.getState()).kind).toBe('confirmDialog');
    expect(core.count('volume_step')).toBe(0);
    expect(useVolume.getState().hud).toBe(false);
  });
});
