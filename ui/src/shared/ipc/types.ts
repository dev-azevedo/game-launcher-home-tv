/**
 * Tipos do contrato `docs/api/launcher-tv.md` (30 comandos, 8 eventos, 26 codigos).
 * Escritos a mao, minimos, ate os tipos gerados do BE-10 (`./generated/`) entrarem: a troca e so
 * reexportar de la neste arquivo. O resto da UI importa tipos apenas de `@/shared/ipc`.
 */
export type GameId = string;
export type MonitorId = string;

export type Origin = 'Steam' | 'Epic' | 'Gog' | 'Manual';
export type SessionState =
  | 'Idle'
  | 'Launching'
  | 'NeedsElevation'
  | 'NotConfirmed'
  | 'Running'
  | 'Ending'
  | 'Closing'
  | 'CloseOverdue';
export type PowerAction = 'Shutdown' | 'Restart' | 'Suspend';
export type Availability = 'Available' | 'Unavailable';
export type VolumeDirection = 'Up' | 'Down';
export type FrontOutcome = 'Front' | 'FrontAfterMinimize' | 'Failed';
export type AutostartMechanism = 'Task' | 'Run' | 'None';
export type DirEntryKind = 'Drive' | 'Dir' | 'Exe';
export type LaunchFailure = 'ElevationDenied';

export interface GameDto {
  id: GameId;
  /** Nome exibido, ja normalizado pelo nucleo. A UI so renderiza como texto (SEC-07). */
  name: string;
  /** Nao nulo quando o usuario renomeou (habilita "Restaurar nome original", CA-62). */
  originalName: string | null;
  origin: Origin;
  /** `cover://<gameId>`; 404 -> placeholder (CA-24). */
  cover: string;
  lastPlayedAt: number | null;
  inProgress: boolean;
}

export interface LibrarySnapshot {
  games: GameDto[];
  focusGameId: GameId | null;
}

export interface SessionDto {
  state: SessionState;
  gameId: GameId | null;
  since: number | null;
  dismissed: boolean;
  elevated: boolean;
  minimized: boolean;
  launchFailure: LaunchFailure | null;
}

export interface VolumeState {
  level: number;
  muted: boolean;
  available: boolean;
}

export interface MonitorDto {
  id: MonitorId;
  name: string;
  primary: boolean;
  current: boolean;
}

export interface SettingsDto {
  autostart: boolean;
  autostartMechanism: AutostartMechanism;
  monitorId: MonitorId | null;
  logDir: string;
}

export interface DirEntry {
  name: string;
  kind: DirEntryKind;
}

export interface DirListing {
  path: string | null;
  parent: string | null;
  entries: DirEntry[];
  /** CT-02: a listagem foi cortada no limite do nucleo. */
  truncated?: boolean;
}

export interface PowerCapabilities {
  shutdown: Availability;
  restart: Availability;
  suspend: Availability;
}

export type PowerTicket = string;

export interface ErrorParams {
  gameId?: GameId;
  origin?: Origin;
}

export interface ErrorDto {
  code: string;
  params?: ErrorParams;
}

/** Os 26 codigos do contrato (secao 5). `errors.test.ts` confere o catalogo contra esta lista. */
export const CONTRACT_ERROR_CODES = [
  'Busy',
  'InvalidRequest',
  'Internal',
  'Duplicate',
  'NotAnExecutable',
  'PathNotAllowed',
  'NetworkPathBlocked',
  'NotFound',
  'AccessDenied',
  'EmptyName',
  'UnknownGame',
  'SessionAlreadyActive',
  'GameFileMissing',
  'StoreNotInstalled',
  'NeedsElevation',
  'ElevationDenied',
  'InvalidState',
  'NoActiveSession',
  'ForegroundDenied',
  'NotOverdue',
  'NoAudioDevice',
  'Unavailable',
  'TicketInvalid',
  'Rejected',
  'AutostartFailed',
  'UnknownMonitor',
] as const;

export type ContractErrorCode = (typeof CONTRACT_ERROR_CODES)[number];

/** Nomes dos 30 comandos (secao 2). */
export const COMMAND_NAMES = [
  'library_get',
  'library_refresh',
  'library_add_manual',
  'library_hidden',
  'game_rename',
  'game_restore_name',
  'game_hide',
  'game_unhide',
  'fs_list_dir',
  'session_get',
  'session_launch',
  'session_launch_with_uac',
  'session_replace',
  'session_return_to_game',
  'session_close',
  'session_force_close',
  'session_retry',
  'session_dismiss',
  'volume_get',
  'volume_step',
  'volume_toggle_mute',
  'power_capabilities',
  'power_prepare',
  'power_execute',
  'settings_get',
  'settings_set_autostart',
  'settings_set_monitor',
  'display_monitors',
  'input_status',
  'app_exit',
] as const;

export type CommandName = (typeof COMMAND_NAMES)[number];

/** Nomes dos 8 eventos (secao 3). */
export const EVENT_NAMES = [
  'input://intent',
  'input://controllers',
  'library://changed',
  'session://state',
  'session://panel',
  'volume://changed',
  'display://changed',
  'system://resumed',
] as const;

export type EventName = (typeof EVENT_NAMES)[number];
