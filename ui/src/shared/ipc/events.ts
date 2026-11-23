import { isInputIntent, type IntentEvent, type IntentSource } from '@/shared/input/intents';
import { listenRaw } from './transport';
import type { FrontOutcome, LibrarySnapshot, MonitorDto, SessionDto, UpdateProgressEvent, VolumeState } from './types';

type Unsubscribe = () => void;

/** Nove eventos do nucleo (docs/api secao 3). Carregam estado completo, nao deltas. */
export const events = {
  controllers: (handler: (connected: number) => void): Unsubscribe =>
    listenRaw('input://controllers', (p) => {
      const connected = (p as { connected?: unknown } | null)?.connected;
      if (typeof connected === 'number' && Number.isInteger(connected) && connected >= 0 && connected <= 4) {
        handler(connected);
      }
    }),
  libraryChanged: (handler: (snapshot: LibrarySnapshot) => void): Unsubscribe =>
    listenRaw('library://changed', (p) => {
      const snap = p as Partial<LibrarySnapshot> | null;
      if (snap && Array.isArray(snap.games)) handler({ games: snap.games, focusGameId: snap.focusGameId ?? null });
    }),
  sessionState: (handler: (session: SessionDto) => void): Unsubscribe =>
    listenRaw('session://state', (p) => {
      if (p && typeof p === 'object' && typeof (p as SessionDto).state === 'string') handler(p as SessionDto);
    }),
  sessionPanel: (handler: (outcome: FrontOutcome) => void): Unsubscribe =>
    listenRaw('session://panel', (p) => {
      const outcome = (p as { outcome?: unknown } | null)?.outcome;
      if (outcome === 'Front' || outcome === 'FrontAfterMinimize' || outcome === 'Failed') handler(outcome);
    }),
  volumeChanged: (handler: (volume: VolumeState) => void): Unsubscribe =>
    listenRaw('volume://changed', (p) => {
      if (p && typeof p === 'object' && typeof (p as VolumeState).level === 'number') handler(p as VolumeState);
    }),
  displayChanged: (handler: (monitors: MonitorDto[]) => void): Unsubscribe =>
    listenRaw('display://changed', (p) => {
      if (Array.isArray(p)) handler(p as MonitorDto[]);
    }),
  systemResumed: (handler: () => void): Unsubscribe => listenRaw('system://resumed', () => handler()),
  updateProgress: (handler: (progress: UpdateProgressEvent) => void): Unsubscribe =>
    listenRaw('update://progress', (p) => {
      const { downloaded, total } = (p ?? {}) as { downloaded?: unknown; total?: unknown };
      if (typeof downloaded !== 'number' || downloaded < 0) return;
      handler({ downloaded, total: typeof total === 'number' && total > 0 ? total : null });
    }),
} as const;

/**
 * Adapter real de `input://intent` para a porta `IntentSource` (arch 3.3). Payload fora do
 * contrato (intencao desconhecida, tipo errado) e descartado: a UI nunca age sobre texto livre.
 */
export function createCoreIntentSource(): IntentSource {
  return {
    subscribe(handler: (event: IntentEvent) => void) {
      return listenRaw('input://intent', (payload) => {
        if (typeof payload !== 'object' || payload === null) return;
        const { intent, repeat } = payload as { intent?: unknown; repeat?: unknown };
        if (!isInputIntent(intent)) return;
        handler({ intent, repeat: repeat === true });
      });
    },
  };
}
