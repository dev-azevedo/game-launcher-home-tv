import { IpcError, toErrorDto } from './errors';
import { invokeRaw } from './transport';
import type {
  CommandName,
  DirListing,
  GameDto,
  GameId,
  LibrarySnapshot,
  MonitorDto,
  MonitorId,
  PowerAction,
  PowerCapabilities,
  PowerTicket,
  SessionDto,
  SettingsDto,
  UpdateDto,
  VolumeDirection,
  VolumeState,
} from './types';

async function call<T>(command: CommandName, args?: Record<string, unknown>): Promise<T> {
  try {
    return (await invokeRaw(command, args)) as T;
  } catch (thrown) {
    throw new IpcError(toErrorDto(thrown));
  }
}

const noValue = (command: CommandName, args?: Record<string, unknown>) => call<void>(command, args);

/**
 * Os 32 comandos do contrato (docs/api/launcher-tv.md secao 2), um por funcao, sem outra porta.
 * Nenhum recebe caminho de executavel, argumento ou PID (SEC-03): so IDs, enums e texto de nome.
 * Falha vira `IpcError` com `ErrorDto`.
 */
export const ipc = {
  // 2.1 Biblioteca
  libraryGet: () => call<LibrarySnapshot>('library_get'),
  libraryRefresh: () => noValue('library_refresh'),
  /** Unica entrada com caminho de arquivo (SEC-09), usada pelo navegador de pastas. */
  libraryAddManual: (path: string) => call<GameDto>('library_add_manual', { path }),
  libraryHidden: () => call<GameDto[]>('library_hidden'),
  gameRename: (gameId: GameId, name: string) => call<GameDto>('game_rename', { gameId, name }),
  gameRestoreName: (gameId: GameId) => call<GameDto>('game_restore_name', { gameId }),
  gameHide: (gameId: GameId) => noValue('game_hide', { gameId }),
  gameUnhide: (gameId: GameId) => noValue('game_unhide', { gameId }),
  /** `null` = unidades locais. */
  fsListDir: (path: string | null) => call<DirListing>('fs_list_dir', { path }),

  // 2.2 Sessao
  sessionGet: () => call<SessionDto>('session_get'),
  sessionLaunch: (gameId: GameId) => noValue('session_launch', { gameId }),
  sessionLaunchWithUac: (gameId: GameId) => noValue('session_launch_with_uac', { gameId }),
  sessionReplace: (gameId: GameId) => noValue('session_replace', { gameId }),
  sessionReturnToGame: () => noValue('session_return_to_game'),
  sessionClose: () => noValue('session_close'),
  sessionForceClose: () => noValue('session_force_close'),
  sessionRetry: () => noValue('session_retry'),
  sessionDismiss: () => noValue('session_dismiss'),

  // 2.3 Volume
  volumeGet: () => call<VolumeState>('volume_get'),
  volumeStep: (direction: VolumeDirection) => call<VolumeState>('volume_step', { direction }),
  volumeToggleMute: () => call<VolumeState>('volume_toggle_mute'),

  // 2.4 Energia
  powerCapabilities: () => call<PowerCapabilities>('power_capabilities'),
  powerPrepare: (action: PowerAction) => call<PowerTicket>('power_prepare', { action }),
  powerExecute: (ticket: PowerTicket) => noValue('power_execute', { ticket }),

  // 2.5 Configuracoes, monitor e app
  settingsGet: () => call<SettingsDto>('settings_get'),
  settingsSetAutostart: (enabled: boolean) => call<SettingsDto>('settings_set_autostart', { enabled }),
  settingsSetMonitor: (monitorId: MonitorId | null) =>
    call<SettingsDto>('settings_set_monitor', { monitorId }),
  displayMonitors: () => call<MonitorDto[]>('display_monitors'),
  inputStatus: () => call<{ connected: number }>('input_status'),
  appExit: () => noValue('app_exit'),

  // 2.6 Atualizacao do app (SEC-33): sem argumentos, a UI nao escolhe URL nem arquivo.
  /** `null` = ja esta na versao mais nova. */
  updateCheck: () => call<UpdateDto | null>('update_check'),
  /** So resolve em erro: em sucesso o launcher fecha e o instalador o reabre. */
  updateInstall: () => noValue('update_install'),
} as const;

export type Ipc = typeof ipc;
