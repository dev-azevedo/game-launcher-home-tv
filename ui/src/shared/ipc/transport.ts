/**
 * Unico arquivo que importa `@tauri-apps/api` (INV-02, SEC-03). Tudo o mais em `src/`
 * fala com o nucleo pela fachada de `index.ts`.
 */
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export function invokeRaw(command: string, args?: Record<string, unknown>): Promise<unknown> {
  return invoke(command, args);
}

/** Assina um evento. Cancelar antes de o `listen` resolver tambem funciona. */
export function listenRaw(event: string, handler: (payload: unknown) => void): () => void {
  let off: (() => void) | null = null;
  let cancelled = false;
  // Fora do WebView (navegador puro) o `listen` falha: sem nucleo nao ha evento, e a UI segue.
  listen<unknown>(event, (e) => handler(e.payload)).then(
    (unlisten) => {
      if (cancelled) unlisten();
      else off = unlisten;
    },
    () => undefined,
  );
  return () => {
    cancelled = true;
    off?.();
    off = null;
  };
}
