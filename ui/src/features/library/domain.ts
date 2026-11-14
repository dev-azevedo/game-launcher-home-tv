import type { GameDto, GameId } from '@/shared/ipc';

/** Tamanhos das fileiras da Home (mockup home.html): Recentes e Biblioteca (A a Z). */
export const RECENTS_MAX = 8;
export const LIBRARY_ROW_MAX = 12;
/** Acima disto a grade de Todos os jogos so monta o que esta perto da tela (arch 3.4). */
export const VIRTUALIZE_ABOVE = 60;

const collator = new Intl.Collator('pt-BR', { sensitivity: 'base', numeric: true });

/** Jogos ja iniciados, do ultimo jogado ao mais antigo (RN-05, CA-22). */
export function recentGames(games: readonly GameDto[], max = RECENTS_MAX): GameDto[] {
  return games
    .filter((g) => g.lastPlayedAt !== null)
    .sort((a, b) => (b.lastPlayedAt ?? 0) - (a.lastPlayedAt ?? 0))
    .slice(0, max);
}

/** Ordem alfabetica sem diferenciar maiusculas e acentos, pelo nome exibido (RN-05). */
export function alphabetical(games: readonly GameDto[]): GameDto[] {
  return [...games].sort((a, b) => collator.compare(a.name, b.name));
}

/** Chaves de foco: o mesmo jogo aparece em Recentes e na Biblioteca, entao as chaves diferem. */
export const focusKeyRecent = (id: GameId) => `r:${id}`;
export const focusKeyLibrary = (id: GameId) => `a:${id}`;
export const focusKeyGrid = (id: GameId) => `g:${id}`;
export const FOCUS_ALL = 'all';
export const FOCUS_ADD = 'add';
export const FOCUS_REFRESH = 'refresh';

/**
 * Foco inicial da Home (CA-22, CA-30): o jogo indicado pelo nucleo (`focusGameId`), senao o ultimo
 * jogado, senao o primeiro de A a Z. Biblioteca vazia: "Adicionar jogo" (CA-23).
 */
export function initialHomeFocus(games: readonly GameDto[], focusGameId: GameId | null): string {
  if (games.length === 0) return FOCUS_ADD;
  const recents = recentGames(games);
  const row = alphabetical(games).slice(0, LIBRARY_ROW_MAX);
  if (focusGameId) {
    if (recents.some((g) => g.id === focusGameId)) return focusKeyRecent(focusGameId);
    if (row.some((g) => g.id === focusGameId)) return focusKeyLibrary(focusGameId);
  }
  const first = recents[0];
  if (first) return focusKeyRecent(first.id);
  return focusKeyLibrary(row[0]!.id);
}

const DAY_MS = 24 * 60 * 60 * 1000;

function startOfDay(ms: number): number {
  const d = new Date(ms);
  return new Date(d.getFullYear(), d.getMonth(), d.getDate()).getTime();
}

/** "Jogado hoje / ontem / ha N dias / Nunca jogado" (microcopy, Geral). Dias de calendario local. */
export function lastPlayedLabel(lastPlayedAt: number | null, now: number): string {
  if (lastPlayedAt === null) return 'Nunca jogado';
  const days = Math.max(0, Math.round((startOfDay(now) - startOfDay(lastPlayedAt)) / DAY_MS));
  if (days === 0) return 'Jogado hoje';
  if (days === 1) return 'Jogado ontem';
  return `Jogado há ${days} dias`;
}
