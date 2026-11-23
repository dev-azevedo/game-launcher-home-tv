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
import type { InputIntent } from '@/shared/input/intents';
import type { GameDto, MonitorDto, SettingsDto } from '@/shared/ipc';
import { emitCoreEvent, mockCore, rejectWith, resetCore } from '@/shared/ipc/testing';
import { APP_VERSION, chosenMonitorMissing, librarySummary, mechanismLabel } from './domain';
import { useSettings } from './store';

const game = (id: string, name: string, origin: GameDto['origin'] = 'Steam'): GameDto => ({
  id: `steam:${id}`,
  name,
  originalName: null,
  origin,
  cover: '',
  lastPlayedAt: null,
  inProgress: false,
});
const SETTINGS: SettingsDto = { autostart: false, autostartMechanism: 'None', monitorId: 'm1', logDir: 'C:\\Logs\\sofa' };
const MONITORS: MonitorDto[] = [
  { id: 'm1', name: 'Monitor Dell', primary: true, current: true },
  { id: 'm2', name: 'TV Sala (HDMI)', primary: false, current: false },
];
const HIDDEN = [game('7', 'Oculto Um', 'Epic'), game('8', 'Oculto Dois')];

const intent = (i: InputIntent) => act(() => void dispatchIntent({ intent: i, repeat: false }, defaultDeps()));
// A chave do foco vem do norigin: no jsdom o atributo `data-focused` do item anterior pode demorar a apagar.
const focusedKey = () => getCurrentFocusKey();
const focusedText = () =>
  Array.from(document.querySelectorAll<HTMLElement>('[data-focused="true"]'))
    .map((e) => e.textContent)
    .join('|');
const kinds = () => useNavigation.getState().stack.map((e) => e.kind);

function core(extra: Parameters<typeof mockCore>[0] = {}) {
  return mockCore({
    settings_get: SETTINGS,
    display_monitors: MONITORS,
    library_hidden: HIDDEN,
    ...extra,
  });
}
async function openSettings() {
  render(<ScreenStack />);
  await waitFor(() => expect(document.querySelectorAll('[data-focused="true"]')).toHaveLength(1));
  act(() => useNavigation.getState().push('settings'));
  await screen.findByRole('tablist', { name: 'Configurações' });
  await waitFor(() => expect(focusedKey()).toBe('tab-screen'));
}
async function goTab(id: string) {
  act(() => void setFocus(`tab-${id}`));
  await waitFor(() => expect(focusedKey()).toBe(`tab-${id}`));
}

beforeEach(() => {
  resetSpatialNavigation();
  useNavigation.getState().reset();
  useSession.setState({ session: IDLE_SESSION, failure: null, replace: null });
  useSettings.setState({ settings: null, monitors: [], hidden: [], loaded: false, autostartFailed: false, savingAutostart: false });
  useLibrary.setState({ status: 'ready', games: [game('1', 'Alfa'), game('2', 'Beta', 'Gog')], notice: null, refreshing: false });
});
afterEach(() => {
  // Desmonta antes de limpar o mock do nucleo: o `unlisten` dos eventos precisa dele.
  cleanup();
  resetCore();
});

describe('regras puras de configuracoes', () => {
  it('resumo da biblioteca por origem', () => {
    expect(librarySummary([])).toBe('0 jogos');
    expect(librarySummary([game('1', 'A')])).toBe('1 jogo: 1 Steam');
    expect(librarySummary([game('1', 'A'), game('2', 'B', 'Gog'), game('3', 'C')])).toBe('3 jogos: 2 Steam, 1 GOG');
  });

  it('monitor escolhido ausente (CA-67): so com preferencia que nao esta na lista', () => {
    expect(chosenMonitorMissing({ ...SETTINGS, monitorId: 'm9' }, MONITORS)).toBe(true);
    expect(chosenMonitorMissing({ ...SETTINGS, monitorId: null }, MONITORS)).toBe(false);
    expect(chosenMonitorMissing(SETTINGS, MONITORS)).toBe(false);
    expect(mechanismLabel('Task')).toBe('Tarefa agendada do Windows');
  });
});

describe('configuracoes (FE-12)', () => {
  it('abre na aba Tela com os monitores, "Em uso" e o aviso de um monitor so', async () => {
    core({ display_monitors: [MONITORS[0]!] });
    await openSettings();
    expect(await screen.findByText('Monitor Dell')).toBeInTheDocument();
    expect(screen.getByText(/Em uso/)).toBeInTheDocument();
    expect(screen.getByText('Só há um monitor conectado.')).toBeInTheDocument();
  });

  it('monitor escolhido ausente mostra o aviso (CA-67)', async () => {
    core({ settings_get: { ...SETTINGS, monitorId: 'm9' } });
    await openSettings();
    expect(await screen.findByText(/O monitor escolhido não está conectado/)).toBeInTheDocument();
  });

  it('A em outro monitor chama settings_set_monitor e avisa (CA-66)', async () => {
    const c = core({ settings_set_monitor: { ...SETTINGS, monitorId: 'm2' } });
    await openSettings();
    await screen.findByText('TV Sala (HDMI)');
    intent('Right');
    await waitFor(() => expect(focusedText()).toContain('Monitor Dell'));
    act(() => void setFocus('menu-m1'));
    await waitFor(() => expect(focusedText()).toContain('TV Sala'));
    intent('Confirm');
    await waitFor(() => expect(c.count('settings_set_monitor')).toBe(1));
    expect(c.calls.find((x) => x.command === 'settings_set_monitor')!.args).toEqual({ monitorId: 'm2' });
    await waitFor(() => expect(useLibrary.getState().notice).toBe('SofaLauncher movido para TV Sala (HDMI).'));
  });

  it('display://changed atualiza a lista com a tela aberta', async () => {
    core();
    await openSettings();
    await screen.findByText('TV Sala (HDMI)');
    await emitCoreEvent('display://changed', [MONITORS[0]]);
    await waitFor(() => expect(screen.queryByText('TV Sala (HDMI)')).toBeNull());
  });

  it('→ entra no painel, ← volta as abas, B no painel volta as abas e B nas abas sai', async () => {
    core();
    await openSettings();
    await screen.findByText('Monitor Dell');
    intent('Right');
    await waitFor(() => expect(focusedKey()).toBe('menu-m0'));
    intent('Left');
    await waitFor(() => expect(focusedKey()).toBe('tab-screen'));
    intent('Confirm');
    await waitFor(() => expect(focusedKey()).toBe('menu-m0'));
    intent('Back');
    await waitFor(() => expect(focusedKey()).toBe('tab-screen'));
    expect(kinds()).toContain('settings');
    intent('Back');
    await waitFor(() => expect(kinds()).toEqual(['home']));
  });

  it('Geral: "Iniciar com o Windows" liga pelo nucleo; mecanismo e pasta de logs so como texto', async () => {
    const c = core({ settings_set_autostart: { ...SETTINGS, autostart: true, autostartMechanism: 'Task' } });
    await openSettings();
    await goTab('general');
    expect(await screen.findByText('C:\\Logs\\sofa')).toBeInTheDocument();
    expect(screen.getByText('Desligada')).toBeInTheDocument();
    expect(screen.getByText(/O SofaLauncher nunca roda como administrador/)).toBeInTheDocument();
    intent('Right');
    await waitFor(() => expect(focusedKey()).toBe('menu-auto'));
    intent('Confirm');
    await waitFor(() => expect(c.count('settings_set_autostart')).toBe(1));
    expect(c.calls.find((x) => x.command === 'settings_set_autostart')!.args).toEqual({ enabled: true });
    await waitFor(() => expect(screen.getByRole('switch', { name: /Iniciar com o Windows/ })).toHaveAttribute('aria-checked', 'true'));
    expect(screen.getByText('Tarefa agendada do Windows')).toBeInTheDocument();
    // Logs e mecanismo nao sao focaveis: so a chave e "Procurar atualizacoes" tem foco no painel.
    expect(Array.from(document.querySelectorAll<HTMLElement>('.pane [data-focus-key]')).map((e) => e.dataset.focusKey)).toEqual([
      'menu-auto',
      'menu-update',
    ]);
  });

  it('Geral: "Procurar atualizacoes" mostra a versao atual e avisa quando ja esta na mais nova', async () => {
    const c = core({ update_check: null });
    await openSettings();
    await goTab('general');
    expect(await screen.findByText(`Você usa a versão ${APP_VERSION}. Precisa de internet.`)).toBeInTheDocument();
    intent('Right');
    await waitFor(() => expect(focusedKey()).toBe('menu-auto'));
    // jsdom sem layout: ↓ espacial nao anda; foca o item direto.
    act(() => void setFocus('menu-update'));
    await waitFor(() => expect(focusedKey()).toBe('menu-update'));
    intent('Confirm');
    await waitFor(() => expect(c.count('update_check')).toBe(1));
    await waitFor(() => expect(useLibrary.getState().notice).toBe('Você já está na versão mais nova.'));
  });

  it('Geral: falha ao alterar mostra a linha de erro e o item segue em foco', async () => {
    core({ settings_set_autostart: rejectWith({ code: 'AutostartFailed' }) });
    await openSettings();
    await goTab('general');
    await screen.findByText('Pasta de logs');
    intent('Right');
    await waitFor(() => expect(focusedKey()).toBe('menu-auto'));
    intent('Confirm');
    expect(await screen.findByRole('alert')).toHaveTextContent('Não foi possível alterar a inicialização com o Windows');
    expect(focusedKey()).toBe('menu-auto');
  });

  it('Biblioteca: resumo, Atualizar chama library_refresh e Adicionar jogo abre o navegador', async () => {
    const c = core({ library_refresh: null, fs_list_dir: { path: null, parent: null, entries: [], truncated: false } });
    await openSettings();
    await goTab('library');
    expect(await screen.findByText('2 jogos: 1 Steam, 1 GOG')).toBeInTheDocument();
    intent('Right');
    await waitFor(() => expect(focusedKey()).toBe('menu-refresh'));
    intent('Confirm');
    await waitFor(() => expect(c.count('library_refresh')).toBe(1));
    useLibrary.setState({ refreshing: false });
    act(() => void setFocus('menu-add'));
    await waitFor(() => expect(focusedKey()).toBe('menu-add'));
    intent('Confirm');
    await waitFor(() => expect(kinds()).toEqual(['home', 'settings', 'folderBrowser']));
  });

  it('Itens ocultos: contagem na aba; A + Restaurar (em foco) devolve o item pelo nucleo (CA-70)', async () => {
    const c = core({ game_unhide: null });
    await openSettings();
    await waitFor(() => expect(document.querySelector('.tab .cnt')).toHaveTextContent('2'));
    await goTab('hidden');
    expect(await screen.findByText('Oculto Um')).toBeInTheDocument();
    intent('Right');
    await waitFor(() => expect(focusedKey()).toBe('menu-h:steam:7'));
    intent('Confirm');
    expect(await screen.findByText('Restaurar Oculto Um?')).toBeInTheDocument();
    await waitFor(() => expect(focusedText()).toBe('Restaurar'));
    c.set({ library_hidden: [HIDDEN[1]!] });
    intent('Confirm');
    await waitFor(() => expect(c.count('game_unhide')).toBe(1));
    expect(c.calls.find((x) => x.command === 'game_unhide')!.args).toEqual({ gameId: 'steam:7' });
    await waitFor(() => expect(useLibrary.getState().notice).toBe('Oculto Um voltou para a biblioteca.'));
    await waitFor(() => expect(screen.queryByText('Oculto Um')).toBeNull());
    // O foco segue para o proximo item do painel.
    await waitFor(() => expect(focusedKey()).toBe('menu-h:steam:8'));
  });

  it('Itens ocultos: Cancelar nao restaura', async () => {
    const c = core({ game_unhide: null });
    await openSettings();
    await goTab('hidden');
    await screen.findByText('Oculto Um');
    intent('Right');
    await waitFor(() => expect(focusedKey()).toBe('menu-h:steam:7'));
    intent('Confirm');
    await screen.findByText('Restaurar Oculto Um?');
    intent('Back');
    await waitFor(() => expect(screen.queryByText('Restaurar Oculto Um?')).toBeNull());
    expect(c.count('game_unhide')).toBe(0);
  });

  it('Itens ocultos vazio: "Nenhum item oculto" e → nao entra no painel', async () => {
    core({ library_hidden: [] });
    await openSettings();
    await goTab('hidden');
    expect(await screen.findByText('Nenhum item oculto')).toBeInTheDocument();
    expect(screen.getByText('Os jogos que você ocultar aparecem aqui.')).toBeInTheDocument();
    intent('Right');
    expect(focusedKey()).toBe('tab-hidden');
  });

  it('nome de item oculto com HTML aparece literal (SEC-07)', async () => {
    const evil = '<img src=x onerror=alert(1)>';
    core({ library_hidden: [game('9', evil)] });
    await openSettings();
    await goTab('hidden');
    expect(await screen.findByText(evil)).toBeInTheDocument();
    expect(document.querySelectorAll('img[onerror]')).toHaveLength(0);
  });
});
