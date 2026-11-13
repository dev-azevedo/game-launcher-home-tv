/** Telas e camadas da pilha (arch 3.1). */
export const SCREEN_KINDS = [
  'home',
  'allGames',
  'gameOptions',
  'renameDialog',
  'quickMenu',
  'confirmDialog',
  'sessionPanel',
  'waiting',
  'settings',
  'folderBrowser',
] as const;

export type ScreenKind = (typeof SCREEN_KINDS)[number];

export interface ScreenEntry {
  /** Chave unica da instancia (a mesma tela pode abrir de novo). */
  key: string;
  kind: ScreenKind;
  /** Chave de foco de quem abriu, para devolver o foco ao fechar a camada. */
  returnFocusKey?: string;
}
