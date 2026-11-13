/** Intencoes emitidas pelo nucleo (`input://intent`) e pelo teclado de emergencia (arch 3.3). */
export const INTENTS = [
  'Up',
  'Down',
  'Left',
  'Right',
  'Confirm',
  'Back',
  'Menu',
  'View',
  'PagePrev',
  'PageNext',
  'VolumeDown',
  'VolumeUp',
] as const;

export type InputIntent = (typeof INTENTS)[number];
export type Direction = 'up' | 'down' | 'left' | 'right';

export interface IntentEvent {
  intent: InputIntent;
  repeat: boolean;
}

export function isInputIntent(value: unknown): value is InputIntent {
  return typeof value === 'string' && (INTENTS as readonly string[]).includes(value);
}

const DIRECTIONS: Partial<Record<InputIntent, Direction>> = {
  Up: 'up',
  Down: 'down',
  Left: 'left',
  Right: 'right',
};

export function intentDirection(intent: InputIntent): Direction | null {
  return DIRECTIONS[intent] ?? null;
}

/** Porta de entrada: o adapter real (`input://intent`) entra em shared/ipc quando o contrato sair. */
export interface IntentSource {
  subscribe(handler: (event: IntentEvent) => void): () => void;
}

/** Fake roteirizado para testes e para a galeria de desenvolvimento. */
export class FakeIntentSource implements IntentSource {
  private handlers = new Set<(event: IntentEvent) => void>();

  subscribe(handler: (event: IntentEvent) => void): () => void {
    this.handlers.add(handler);
    return () => this.handlers.delete(handler);
  }

  emit(intent: InputIntent, repeat = false): void {
    for (const handler of [...this.handlers]) handler({ intent, repeat });
  }
}

/** Junta varias fontes (nucleo + teclado) numa so. */
export function mergeSources(...sources: IntentSource[]): IntentSource {
  return {
    subscribe(handler) {
      const offs = sources.map((s) => s.subscribe(handler));
      return () => offs.forEach((off) => off());
    },
  };
}
