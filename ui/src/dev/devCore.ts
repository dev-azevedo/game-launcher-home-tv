import { emitCoreEvent, mockCore } from '@/shared/ipc/testing';
import type { GameDto, Origin, SessionDto } from '@/shared/ipc';

/**
 * Nucleo simulado SO para o navegador de desenvolvimento (sem Tauri): permite conferir a UI em
 * 1080p e 4K. Fora do build de producao (`import.meta.env.DEV`). Cenarios pela URL, ex.:
 * `#lib=muitos&ctrl=none`. Jogos "especiais" simulam erros do contrato ao apertar A.
 */
const NAMES = [
  'Vale dos Ventos', 'Minério', 'Guarda-Chuva Noturno', 'Bruma', 'Última Parada', 'Rota 66',
  'Noite Eterna', 'Ferrugem', 'Cidade Submersa', 'Orquestra de Cinzas', 'Farol', 'Gelo Fino', 'Quarto Escuro',
];
const LONG = [
  '<img src=x onerror=alert(1)>',
  'Um título extraordinariamente longo para testar o corte do cartaz na fileira e o herói em duas linhas',
  'ÁÉÍ Ação Ágil', 'A', 'Zeta',
];
const ORIGINS: Origin[] = ['Steam', 'Epic', 'Gog', 'Manual'];

function makeGames(kind: string): GameDto[] {
  if (kind === 'vazia' || kind === 'carregando') return [];
  const count = kind === 'muitos' ? 500 : kind === 'unico' ? 1 : kind === 'longo' ? LONG.length + NAMES.length : NAMES.length;
  const names = kind === 'longo' ? [...LONG, ...NAMES] : NAMES;
  const now = Date.now();
  return Array.from({ length: count }, (_, i) => {
    const name = i < names.length ? names[i]! : `Jogo ${String(i).padStart(3, '0')}`;
    const id = `steam:${1000 + i}`;
    return {
      id,
      name,
      originalName: i === 0 ? 'Vale do Vento' : null,
      origin: ORIGINS[i % ORIGINS.length]!,
      cover: `cover://${id}`,
      lastPlayedAt: i < 5 ? now - i * 26 * 3600 * 1000 : null,
      inProgress: false,
    };
  });
}

export async function installDevCore(): Promise<void> {
  const params = new URLSearchParams(window.location.hash.replace(/^#/, '').replace(/^gallery&?/, ''));
  const lib = params.get('lib') ?? 'cheia';
  let games = makeGames(lib);
  const snapshot = () => ({ games, focusGameId: null });
  let session: SessionDto = { state: 'Idle', gameId: null, since: null, dismissed: false, elevated: false, minimized: false, launchFailure: null };
  const setSession = (patch: Partial<SessionDto>) => {
    session = { ...session, ...patch, since: Date.now() };
    void emitCoreEvent('session://state', session);
  };

  mockCore({
    library_get: () => snapshot(),
    library_refresh: () => {
      setTimeout(() => void emitCoreEvent('library://changed', snapshot()), 1200);
    },
    library_hidden: () => [],
    session_get: () => session,
    session_launch: ({ gameId }) => {
      const game = games.find((g) => g.id === gameId);
      if (session.state !== 'Idle') return Promise.reject({ code: 'SessionAlreadyActive', params: { gameId: session.gameId } });
      if (game?.name === 'Minério') return Promise.reject({ code: 'NetworkPathBlocked' });
      if (game?.name === 'Bruma') return Promise.reject({ code: 'GameFileMissing', params: { gameId } });
      if (game?.name === 'Guarda-Chuva Noturno') {
        setSession({ state: 'NeedsElevation', gameId: gameId as string, dismissed: false, elevated: false, launchFailure: null });
        return Promise.reject({ code: 'NeedsElevation', params: { gameId } });
      }
      setSession({ state: 'Launching', gameId: gameId as string, dismissed: false, elevated: false, launchFailure: null });
      if (game?.name !== 'Última Parada') {
        setTimeout(() => {
          games = games.map((g) => (g.id === gameId ? { ...g, inProgress: true, lastPlayedAt: Date.now() } : g));
          void emitCoreEvent('library://changed', snapshot());
          setSession({ state: 'Running' });
        }, 6000);
      } else {
        setTimeout(() => setSession({ state: 'NotConfirmed' }), 4000);
      }
      return undefined;
    },
    session_launch_with_uac: () => setSession({ state: 'Launching', elevated: true }),
    session_dismiss: () => {
      if (session.state === 'Launching') setSession({ dismissed: true });
      else if (session.state === 'NotConfirmed' || session.state === 'NeedsElevation') {
        setSession({ state: 'Idle', gameId: null, launchFailure: null });
      }
    },
    session_retry: () => setSession({ state: 'Launching', dismissed: false }),
    game_hide: ({ gameId }) => {
      games = games.filter((g) => g.id !== gameId);
      void emitCoreEvent('library://changed', snapshot());
    },
    game_rename: ({ gameId, name }) => {
      const trimmed = String(name).trim();
      if (!trimmed) return Promise.reject({ code: 'EmptyName' });
      games = games.map((g) => (g.id === gameId ? { ...g, name: trimmed, originalName: g.originalName ?? g.name } : g));
      void emitCoreEvent('library://changed', snapshot());
      return games.find((g) => g.id === gameId);
    },
    game_restore_name: ({ gameId }) => {
      games = games.map((g) => (g.id === gameId && g.originalName ? { ...g, name: g.originalName, originalName: null } : g));
      void emitCoreEvent('library://changed', snapshot());
      return games.find((g) => g.id === gameId);
    },
    input_status: () => ({ connected: params.get('ctrl') === 'none' ? 0 : 1 }),
  });

  // Roteiro de controle para conferir o aviso (CA-14, CA-15): `#ctrl=lost`.
  if (params.get('ctrl') === 'lost') {
    setTimeout(() => void emitCoreEvent('input://controllers', { connected: 0 }), 1500);
    setTimeout(() => void emitCoreEvent('input://controllers', { connected: 1 }), 6000);
  }
}
