import type { DirEntry } from '@/shared/ipc';

const SEP = '\\';

/** Caminho de uma entrada. `dir === null` = unidades: o nome da unidade ja e o caminho (ex.: `C:\`). */
export function entryPath(dir: string | null, entry: DirEntry): string {
  if (dir === null || entry.kind === 'Drive') return entry.name;
  return dir.endsWith(SEP) ? `${dir}${entry.name}` : `${dir}${SEP}${entry.name}`;
}

/** Nome sugerido ao cadastrar: o do arquivo, sem a extensao `.exe` (CA-19). */
export function gameNameFromExe(fileName: string): string {
  return fileName.replace(/\.exe$/i, '');
}

/**
 * Para onde "B" leva: a pasta de cima; na raiz de uma unidade, a lista de unidades (`null`); na lista de
 * unidades, nenhum lugar (`undefined` = sai do navegador). O nucleo informa `parent`; `null` com caminho = raiz.
 */
export function parentTarget(path: string | null, parent: string | null): string | null | undefined {
  if (path === null) return undefined;
  return parent ?? null;
}

/** Ultimo trecho do caminho, para devolver o foco a pasta de onde o usuario voltou. */
export function lastSegment(path: string): string {
  const trimmed = path.replace(/\\+$/, '');
  return trimmed.slice(trimmed.lastIndexOf(SEP) + 1);
}
