import { afterEach, describe, expect, it, vi } from 'vitest';
import { clearMocks, mockConvertFileSrc } from '@tauri-apps/api/mocks';
import { COMMAND_NAMES, coverUrl, createCoreIntentSource, events, EVENT_NAMES, ipc, IpcError, toErrorDto } from '.';
import { emitCoreEvent, mockCore, rejectWith, resetCore } from './testing';

afterEach(() => resetCore());

const flush = () => new Promise((r) => setTimeout(r, 0));

describe('comandos', () => {
  it('a fachada cobre os 30 comandos do contrato, cada um chamando o nome certo', async () => {
    const core = mockCore(Object.fromEntries(COMMAND_NAMES.map((c) => [c, null])));
    await ipc.libraryGet();
    await ipc.libraryRefresh();
    await ipc.libraryAddManual('D:\\Jogos\\a.exe');
    await ipc.libraryHidden();
    await ipc.gameRename('steam:620', 'Portal');
    await ipc.gameRestoreName('steam:620');
    await ipc.gameHide('steam:620');
    await ipc.gameUnhide('steam:620');
    await ipc.fsListDir(null);
    await ipc.sessionGet();
    await ipc.sessionLaunch('steam:620');
    await ipc.sessionLaunchWithUac('steam:620');
    await ipc.sessionReplace('steam:620');
    await ipc.sessionReturnToGame();
    await ipc.sessionClose();
    await ipc.sessionForceClose();
    await ipc.sessionRetry();
    await ipc.sessionDismiss();
    await ipc.volumeGet();
    await ipc.volumeStep('Up');
    await ipc.volumeToggleMute();
    await ipc.powerCapabilities();
    await ipc.powerPrepare('Shutdown');
    await ipc.powerExecute('ticket');
    await ipc.settingsGet();
    await ipc.settingsSetAutostart(true);
    await ipc.settingsSetMonitor(null);
    await ipc.displayMonitors();
    await ipc.inputStatus();
    await ipc.appExit();
    expect(COMMAND_NAMES).toHaveLength(30);
    expect(core.calls.map((c) => c.command).sort()).toEqual([...COMMAND_NAMES].sort());
  });

  it('manda os argumentos em camelCase, sem campo extra', async () => {
    const core = mockCore({ game_rename: { id: 'steam:620' }, session_launch: null });
    await ipc.gameRename('steam:620', 'Portal 2');
    await ipc.sessionLaunch('epic:SampleGameA');
    expect(core.calls[0]).toEqual({ command: 'game_rename', args: { gameId: 'steam:620', name: 'Portal 2' } });
    expect(core.calls[1]).toEqual({ command: 'session_launch', args: { gameId: 'epic:SampleGameA' } });
  });

  it('rejeicao vira IpcError com o ErrorDto do contrato', async () => {
    mockCore({ session_launch: rejectWith({ code: 'SessionAlreadyActive', params: { gameId: 'steam:620' } }) });
    const err = await ipc.sessionLaunch('epic:SampleGameA').catch((e: unknown) => e);
    expect(err).toBeInstanceOf(IpcError);
    expect((err as IpcError).dto).toEqual({ code: 'SessionAlreadyActive', params: { gameId: 'steam:620' } });
  });

  it('rejeicao fora do formato (texto tecnico) vira Internal sem vazar o texto', async () => {
    mockCore({ library_get: () => Promise.reject('thread panicked at C:\\Users\\x\\secret.rs') });
    const err = (await ipc.libraryGet().catch((e: unknown) => e)) as IpcError;
    expect(err.dto).toEqual({ code: 'Internal' });
    expect(err.message).not.toContain('secret');
  });
});

describe('toErrorDto', () => {
  it('descarta params malformados ou com caminho', () => {
    expect(toErrorDto({ code: 'GameFileMissing', params: { gameId: 'gog:1000000001', path: 'C:\\x' } })).toEqual({
      code: 'GameFileMissing',
      params: { gameId: 'gog:1000000001' },
    });
    expect(toErrorDto({ code: 'X', params: { gameId: '../../etc', origin: 'Nope' } })).toEqual({ code: 'X' });
    expect(toErrorDto({ code: 123 })).toEqual({ code: 'Internal' });
    expect(toErrorDto(null)).toEqual({ code: 'Internal' });
  });
});

describe('eventos', () => {
  it('input://intent: so intencoes do contrato chegam a UI, com repeat', async () => {
    mockCore();
    const got: Array<[string, boolean]> = [];
    const off = createCoreIntentSource().subscribe((e) => got.push([e.intent, e.repeat]));
    await flush();
    await emitCoreEvent('input://intent', { intent: 'Down', repeat: true });
    await emitCoreEvent('input://intent', { intent: 'Confirm', repeat: false });
    await emitCoreEvent('input://intent', { intent: 'FormatDisk', repeat: false });
    await emitCoreEvent('input://intent', 'Up');
    await emitCoreEvent('input://intent', { intent: 'Back' });
    expect(got).toEqual([
      ['Down', true],
      ['Confirm', false],
      ['Back', false],
    ]);
    off();
    await emitCoreEvent('input://intent', { intent: 'Menu', repeat: false });
    expect(got).toHaveLength(3);
  });

  it('cobre os 8 eventos do contrato e entrega o estado completo', async () => {
    mockCore();
    expect(EVENT_NAMES).toHaveLength(8);
    const spy = {
      controllers: vi.fn(),
      library: vi.fn(),
      session: vi.fn(),
      panel: vi.fn(),
      volume: vi.fn(),
      display: vi.fn(),
      resumed: vi.fn(),
    };
    const offs = [
      events.controllers(spy.controllers),
      events.libraryChanged(spy.library),
      events.sessionState(spy.session),
      events.sessionPanel(spy.panel),
      events.volumeChanged(spy.volume),
      events.displayChanged(spy.display),
      events.systemResumed(spy.resumed),
    ];
    await flush();
    await emitCoreEvent('input://controllers', { connected: 2 });
    await emitCoreEvent('input://controllers', { connected: 9 });
    await emitCoreEvent('library://changed', { games: [], focusGameId: 'steam:620' });
    await emitCoreEvent('session://state', { state: 'Running', gameId: 'steam:620' });
    await emitCoreEvent('session://panel', { open: true, outcome: 'FrontAfterMinimize' });
    await emitCoreEvent('volume://changed', { level: 40, muted: false, available: true });
    await emitCoreEvent('display://changed', []);
    await emitCoreEvent('system://resumed', {});
    expect(spy.controllers).toHaveBeenCalledTimes(1);
    expect(spy.controllers).toHaveBeenCalledWith(2);
    expect(spy.library).toHaveBeenCalledWith({ games: [], focusGameId: 'steam:620' });
    expect(spy.session).toHaveBeenCalledTimes(1);
    expect(spy.panel).toHaveBeenCalledWith('FrontAfterMinimize');
    expect(spy.volume).toHaveBeenCalledTimes(1);
    expect(spy.display).toHaveBeenCalledWith([]);
    expect(spy.resumed).toHaveBeenCalledTimes(1);
    offs.forEach((off) => off());
  });
});

describe('coverUrl', () => {
  afterEach(() => clearMocks());

  it('no Tauri (Windows) monta http://cover.localhost com o GameId codificado', () => {
    mockConvertFileSrc('windows');
    expect(coverUrl('steam:620')).toBe('http://cover.localhost/steam%3A620');
    expect(coverUrl('epic:Fortnite/x y')).toBe('http://cover.localhost/epic%3AFortnite%2Fx%20y');
    expect(coverUrl('manual:00ab12ef')).toBe('http://cover.localhost/manual%3A00ab12ef');
  });

  it('fora do Tauri cai em cover://<id> codificado', () => {
    expect(coverUrl('steam:620')).toBe('cover://steam%3A620');
  });
});
