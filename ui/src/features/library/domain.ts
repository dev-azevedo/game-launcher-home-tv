import type { GameDto, GameId } from '@/shared/ipc';

/** Tamanhos das fileiras da Home (mockup home.html): Recentes e Biblioteca (A a Z). */
export const RECENTS_MAX = 8;
export const LIBRARY_ROW_MAX = 12;
/** Acima disto a grade de Todos os jogos so monta o que esta perto da tela (arch 3.4). */
export const VIRTUALIZE_ABOVE = 60;

/** Jogo pela id (um so lugar para a busca: `selectGameById` e `gameName` usam esta funcao). */
export function findGame(games: readonly GameDto[], id: GameId | null | undefined): GameDto | undefined {
  return id ? games.find((g) => g.id === id) : undefined;
}

/** Nome exibido do jogo, ou o texto de reserva de quem chama ("o jogo", "O jogo"...). */
export function gameName(games: readonly GameDto[], id: GameId | null | undefined, fallback: string): string {
  return findGame(games, id)?.name ?? fallback;
}

/**
 * Chave A-Z igual a `sort_key` do nucleo (`library/model.rs`): NFD, sem marcas combinantes, em
 * minusculas. A ordem comparada por codigo de caractere (como o `String::cmp` do Rust), sem `Intl`,
 * para a fileira A-Z concordar com a ordem do nucleo (RN-05).
 */
export function sortKey(name: string): string {
  return name.normalize('NFD').replace(/\p{M}/gu, '').toLowerCase();
}

function compareKeys(a: string, b: string): number {
  const x = Array.from(a);
  const y = Array.from(b);
  const n = Math.min(x.length, y.length);
  for (let i = 0; i < n; i++) {
    const d = x[i]!.codePointAt(0)! - y[i]!.codePointAt(0)!;
    if (d !== 0) return d;
  }
  return x.length - y.length;
}

/**
 * Jogos ja iniciados, do ultimo jogado ao mais antigo. A lista do nucleo ja vem ordenada pela
 * RN-05 (jogados primeiro, do mais recente ao mais antigo): aqui so filtra e corta, sem reordenar.
 */
export function recentGames(games: readonly GameDto[], max = RECENTS_MAX): GameDto[] {
  return games.filter((g) => g.lastPlayedAt !== null).slice(0, max);
}

/**
 * Fileira A-Z (jogados e nunca jogados misturados): unico caso em que a UI ordena, pelo mesmo
 * criterio do nucleo (nome exibido, sem caixa e acento, empate pela id).
 */
export function alphabetical(games: readonly GameDto[]): GameDto[] {
  return [...games].sort((a, b) => compareKeys(sortKey(a.name), sortKey(b.name)) || compareKeys(a.id, b.id));
}

/**
 * Fileira unica do layout Vitrine: Recentes e depois a fileira A-Z sem repetir jogo. Usa a mesma fileira
 * A-Z da Home (`LIBRARY_ROW_MAX`), entao todo jogo que `initialHomeFocus` pode apontar esta aqui.
 */
export function stripGames(recents: readonly GameDto[], libraryRow: readonly GameDto[]): GameDto[] {
  const seen = new Set(recents.map((g) => g.id));
  return [...recents, ...libraryRow.filter((g) => !seen.has(g.id))];
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
