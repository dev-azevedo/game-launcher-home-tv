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
import type { ErrorCode, ErrorParams } from './generated';

/**
 * `code` e `string` (e nao `ErrorCode`) de proposito: a UI recebe o payload sem validacao e codigo
 * fora da tabela cai em "Desconhecido" (errors.ts). O `ErrorDto` gerado estreita para os 26 codigos.
 */
export interface ErrorDto {
  code: string;
  params?: ErrorParams;
}

/**
 * Os codigos do contrato (secao 5). O `Record<ErrorCode, true>` obriga o compilador a conferir a
 * lista contra o tipo gerado: codigo novo ou removido no nucleo quebra o typecheck aqui.
 * (`ErrorCode` e so tipo; a lista de runtime sai desta tabela, nao de uma copia a mao.)
 */
const CODE_TABLE: Record<ErrorCode, true> = {
  Busy: true,
  InvalidRequest: true,
  Internal: true,
  Duplicate: true,
  NotAnExecutable: true,
  PathNotAllowed: true,
  NetworkPathBlocked: true,
  NotFound: true,
  AccessDenied: true,
  EmptyName: true,
  UnknownGame: true,
  SessionAlreadyActive: true,
  GameFileMissing: true,
  StoreNotInstalled: true,
  NeedsElevation: true,
  ElevationDenied: true,
  InvalidState: true,
  NoActiveSession: true,
  ForegroundDenied: true,
  NotOverdue: true,
  NoAudioDevice: true,
  Unavailable: true,
  TicketInvalid: true,
  Rejected: true,
  AutostartFailed: true,
  UnknownMonitor: true,
};

export const CONTRACT_ERROR_CODES = Object.keys(CODE_TABLE) as ErrorCode[];

export type ContractErrorCode = ErrorCode;

/** Nomes dos 30 comandos (secao 2): vem de `generated/commandNames.ts` (fonte unica: `command_list.rs`). */
export { COMMAND_NAMES } from './generated/commandNames';
export type { CommandName } from './generated/commandNames';

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
