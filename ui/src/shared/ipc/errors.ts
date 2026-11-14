import type { ErrorDto, ErrorParams, GameId, Origin } from './types';

const ORIGINS: readonly string[] = ['Steam', 'Epic', 'Gog', 'Manual'];
const GAME_ID = /^(steam|epic|gog|manual):[A-Za-z0-9_.-]{1,128}$/;

/**
 * Normaliza qualquer rejeicao de `invoke` em `ErrorDto`. O nucleo so manda `{ code, params? }`;
 * qualquer outra forma (string, Error, objeto estranho) vira `Internal`, e o texto original
 * nunca chega a UI (SEC-06). `params` so passa `gameId` e `origin` bem formados.
 */
export function toErrorDto(error: unknown): ErrorDto {
  if (typeof error !== 'object' || error === null) return { code: 'Internal' };
  const { code, params } = error as { code?: unknown; params?: unknown };
  if (typeof code !== 'string' || code.length === 0 || code.length > 64) return { code: 'Internal' };
  const clean = sanitizeParams(params);
  return clean ? { code, params: clean } : { code };
}

function sanitizeParams(params: unknown): ErrorParams | undefined {
  if (typeof params !== 'object' || params === null) return undefined;
  const { gameId, origin } = params as { gameId?: unknown; origin?: unknown };
  const out: ErrorParams = {};
  if (typeof gameId === 'string' && GAME_ID.test(gameId)) out.gameId = gameId as GameId;
  if (typeof origin === 'string' && ORIGINS.includes(origin)) out.origin = origin as Origin;
  return out.gameId || out.origin ? out : undefined;
}

/** Rejeicao de comando como excecao tipada: toda falha da fachada sai como `IpcError`. */
export class IpcError extends Error {
  readonly dto: ErrorDto;
  constructor(dto: ErrorDto) {
    super(dto.code);
    this.name = 'IpcError';
    this.dto = dto;
  }
  get code(): string {
    return this.dto.code;
  }
}

/** Le o `ErrorDto` de qualquer coisa capturada num `catch`. */
export function errorOf(thrown: unknown): ErrorDto {
  return thrown instanceof IpcError ? thrown.dto : toErrorDto(thrown);
}
