import type { UpdateDto } from '@/shared/ipc';

/** Espera depois de abrir o launcher antes de procurar versao nova (nao disputa a primeira varredura). */
export const STARTUP_CHECK_DELAY_MS = 8000;

/** Limite das notas do release no dialogo (texto puro, SEC-07). */
const NOTES_MAX_LINES = 4;
const NOTES_MAX_CHARS = 280;

/** "11,4 MB" (pt-BR, uma casa). */
export function megabytes(bytes: number): string {
  return `${(bytes / (1024 * 1024)).toLocaleString('pt-BR', { minimumFractionDigits: 1, maximumFractionDigits: 1 })} MB`;
}

/** Texto do progresso: "45% · 5,2 de 11,4 MB" ou so "5,2 MB" sem total. */
export function progressLabel(progress: { downloaded: number; total: number | null } | null): string {
  if (!progress) return 'Preparando o download…';
  const { downloaded, total } = progress;
  if (!total) return `${megabytes(downloaded)} baixados`;
  const pct = Math.min(100, Math.floor((downloaded / total) * 100));
  return `${pct}% · ${megabytes(downloaded)} de ${megabytes(total)}`;
}

/**
 * Notas do release em poucas linhas, sem marcacao de Markdown. O texto vem do GitHub e e mostrado
 * como texto (React escapa); aqui so se encurta.
 */
export function shortNotes(update: UpdateDto): string[] {
  if (!update.notes) return [];
  const lines = update.notes
    .split(/\r?\n/)
    .map((l) => l.replace(/^\s*(?:[-*+]|#{1,6})\s+/, '').replace(/[*_`]/g, '').trim())
    .filter((l) => l.length > 0);
  const out: string[] = [];
  let used = 0;
  for (const line of lines.slice(0, NOTES_MAX_LINES)) {
    if (used + line.length > NOTES_MAX_CHARS) {
      out.push(`${line.slice(0, Math.max(0, NOTES_MAX_CHARS - used))}…`);
      return out;
    }
    out.push(line);
    used += line.length;
  }
  if (lines.length > NOTES_MAX_LINES) out[out.length - 1] += ' …';
  return out;
}
