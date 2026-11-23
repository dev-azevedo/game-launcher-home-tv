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
  'launchFlow',
  'settings',
  'folderBrowser',
] as const;

export type ScreenKind = (typeof SCREEN_KINDS)[number];

/** Dados que a camada precisa para abrir (so IDs e enums, nunca caminho: SEC-03). */
export interface ScreenParams {
  gameId?: string;
  /** Qual confirmacao o `confirmDialog` mostra. */
  confirm?: 'hide' | 'close' | 'power' | 'exit' | 'restore' | 'update';
  /** Acao de energia do `confirmDialog` `power` (so o enum, nunca texto livre). */
  power?: 'Shutdown' | 'Restart' | 'Suspend';
}

export interface ScreenEntry {
  /** Chave unica da instancia (a mesma tela pode abrir de novo). */
  key: string;
  kind: ScreenKind;
  /** Chave de foco de quem abriu, para devolver o foco ao fechar a camada. */
  returnFocusKey?: string;
  params?: ScreenParams;
}
