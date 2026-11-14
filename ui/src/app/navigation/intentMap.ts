import { intentDirection, type Direction, type InputIntent } from '@/shared/input/intents';
import type { ScreenKind } from './screens';

/** Acao declarativa resolvida a partir de (tela do topo, intencao). Sem efeito colateral: testavel. */
export type ResolvedAction =
  | { type: 'navigate'; direction: Direction }
  | { type: 'activate' }
  | { type: 'back' }
  | { type: 'open'; screen: ScreenKind }
  | { type: 'page'; delta: -1 | 1; rows: number }
  | { type: 'volume'; direction: 'Up' | 'Down' };

type Table = Partial<Record<InputIntent, ResolvedAction>>;

const move = (intent: InputIntent): ResolvedAction | undefined => {
  const direction = intentDirection(intent);
  return direction ? { type: 'navigate', direction } : undefined;
};

/** Base: direcoes, A e B. Intencoes ausentes da tabela da tela sao ignoradas. */
const base: Table = {
  Up: move('Up'),
  Down: move('Down'),
  Left: move('Left'),
  Right: move('Right'),
  Confirm: { type: 'activate' },
  Back: { type: 'back' },
};

const triggers: Table = {
  VolumeDown: { type: 'volume', direction: 'Down' },
  VolumeUp: { type: 'volume', direction: 'Up' },
};

/** Handoff secao 4 e arch 3.3. Gatilhos (volume) so valem na Home e em Todos os jogos (CA-65). */
const TABLES: Record<ScreenKind, Table> = {
  home: {
    ...base,
    ...triggers,
    Back: undefined, // CA-10: a tela inicial ignora B
    View: { type: 'open', screen: 'gameOptions' },
    Menu: { type: 'open', screen: 'quickMenu' },
    PagePrev: { type: 'page', delta: -1, rows: 1 },
    PageNext: { type: 'page', delta: 1, rows: 1 },
  },
  allGames: {
    ...base,
    ...triggers,
    View: { type: 'open', screen: 'gameOptions' },
    Menu: { type: 'open', screen: 'quickMenu' },
    PagePrev: { type: 'page', delta: -1, rows: 3 },
    PageNext: { type: 'page', delta: 1, rows: 3 },
  },
  gameOptions: { ...base },
  renameDialog: { ...base },
  quickMenu: { ...base, Menu: { type: 'back' } },
  confirmDialog: { ...base },
  sessionPanel: { ...base, Menu: { type: 'open', screen: 'quickMenu' } },
  // RN-31: telas de espera so aceitam A (no botao) e B.
  waiting: { Confirm: base.Confirm, Back: base.Back },
  // Camada de sessao (Abrindo, erros, administrador): mesmo conjunto de dialogo.
  launchFlow: { ...base },
  settings: { ...base },
  folderBrowser: { ...base },
};

export function resolveIntent(screen: ScreenKind, intent: InputIntent): ResolvedAction | null {
  return TABLES[screen][intent] ?? null;
}
