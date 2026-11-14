/**
 * Tipos do contrato `docs/api/launcher-tv.md` (30 comandos, 8 eventos, 26 codigos). Os DTOs vem de
 * `./generated/` (BE-10, gerado, nao editar). O resto da UI importa tipos apenas de `@/shared/ipc`.
 */
export type {
  AutostartMechanism,
  Availability,
  DirEntry,
  DirEntryKind,
  DirListing,
  FrontOutcome,
  GameDto,
  GameId,
  LaunchFailure,
  LibrarySnapshot,
  MonitorDto,
  MonitorId,
  Origin,
  PowerAction,
  PowerCapabilities,
  PowerTicket,
  SessionDto,
  SessionPanelEvent,
  SessionState,
  SettingsDto,
  VolumeDirection,
  VolumeState,
} from './generated';
export type { ErrorCode, ErrorParams } from './generated';
import type { ErrorParams } from './generated';

/**
 * `code` e `string` (e nao `ErrorCode`) de proposito: a UI recebe o payload sem validacao e codigo
 * fora da tabela cai em "Desconhecido" (errors.ts). O `ErrorDto` gerado estreita para os 26 codigos.
 */
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
