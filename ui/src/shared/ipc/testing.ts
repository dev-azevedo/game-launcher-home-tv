/**
 * Harness de testes: `mockIPC` do Tauri com eventos simulados. Importado so por testes
 * (e a unica ponte de teste para `@tauri-apps/api`, que a regra de ESLint restringe a esta pasta).
 */
import { emit } from '@tauri-apps/api/event';
import { clearMocks, mockIPC } from '@tauri-apps/api/mocks';
import type { CommandName, ErrorDto, EventName } from './types';

type Handler = (args: Record<string, unknown>) => unknown | Promise<unknown>;
type StaticValue = null | string | number | boolean | unknown[] | { [key: string]: unknown };
export type CoreHandlers = Partial<Record<CommandName, Handler | StaticValue>>;

export interface CoreMock {
  /** Comandos chamados, em ordem, com os argumentos. */
  calls: Array<{ command: string; args: Record<string, unknown> }>;
  count(command: CommandName): number;
  /** Troca ou acrescenta respostas no meio do teste. */
  set(handlers: CoreHandlers): void;
}

/** Faz um comando rejeitar como o nucleo faz: `{ code, params? }`. */
export function rejectWith(dto: ErrorDto): Handler {
  return () => Promise.reject(dto);
}

/** Instala o mock. Comando sem resposta registrada rejeita com `Internal`. */
export function mockCore(initial: CoreHandlers = {}): CoreMock {
  let handlers: CoreHandlers = { ...initial };
  const calls: CoreMock['calls'] = [];
  mockIPC(
    (cmd, payload) => {
      calls.push({ command: cmd, args: (payload ?? {}) as Record<string, unknown> });
      const h = handlers[cmd as CommandName];
      if (h === undefined) return Promise.reject({ code: 'Internal' });
      return typeof h === 'function' ? (h as Handler)((payload ?? {}) as Record<string, unknown>) : h;
    },
    { shouldMockEvents: true },
  );
  return {
    calls,
    count: (command) => calls.filter((c) => c.command === command).length,
    set: (more) => {
      handlers = { ...handlers, ...more };
    },
  };
}

/** Emite um evento do nucleo para a UI. */
export function emitCoreEvent(event: EventName, payload: unknown): Promise<void> {
  return emit(event, payload);
}

export function resetCore(): void {
  clearMocks();
}
