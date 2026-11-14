import { emitCoreEvent, mockCore, type CoreHandlers } from '@/shared/ipc/testing';
import type {
  DirListing,
  FrontOutcome,
  GameDto,
  MonitorDto,
  Origin,
  PowerCapabilities,
  SessionDto,
  SettingsDto,
  VolumeState,
} from '@/shared/ipc';

/**
 * Nucleo simulado SO para o navegador de desenvolvimento (sem Tauri): permite conferir a UI em
 * 1080p e 4K. Fora do build de producao (`import.meta.env.DEV`). Cenarios pela URL, ex.:
 * `#lib=muitos&ctrl=none`. Jogos "especiais" simulam erros do contrato ao apertar A.
 *
 * Parametros do hash (separados por `&`):
 * - biblioteca e controle: `lib=cheia|muitos|vazia|unico|longo|carregando`, `ctrl=none|lost`
 * - sessao: `start=running` (Steam 1000 em andamento) ou `start=panel` (abre o painel); `trava=yes` (o jogo
 *   ignora o pedido de fechar e vai a CloseOverdue); `fg=denied` (voltar ao jogo recusado); `force=denied`
 *   (jogo administrador); `fast=1` (os 10 s viram 3 s). Fora do contrato de eventos, F2 abre o painel.
 * - menu rapido: `audio=none` (sem dispositivo), `suspend=no` (suspensao indisponivel), `power=fail`
 *   (Windows recusa), `autostart=on|off`
 * - configuracoes: `monitors=um|dois|ausente`, `hidden=nenhum|muitos`, `autostart=fail` (nao liga)
 * - navegador de pastas: `fs=muitos` (3000 arquivos, lista cortada), `fs=vazio` (raiz sem unidades)
 * Console (so DEV): `__devCore.panel('FrontAfterMinimize')`, `__devCore.resume()`, `__devCore.displayChanged()`.
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

/** Sistema de arquivos de mentira para o navegador de pastas (nomes crus, como o nucleo manda, CT-03). */
const D = (name: string, kind: 'Drive' | 'Dir' | 'Exe') => ({ name, kind });
const FS: Record<string, Array<ReturnType<typeof D>>> = {
  'C:\\': [D('Jogos', 'Dir'), D('Program Files', 'Dir'), D('Users', 'Dir'), D('instalador-antigo.exe', 'Exe')],
  'C:\\Jogos': [D('Aventura', 'Dir'), D('Aventura.exe', 'Exe'), D('Bruma.exe', 'Exe'), D('leiame.exe', 'Exe')],
  'C:\\Jogos\\Aventura': [D('Aventura.exe', 'Exe'), D('Config.exe', 'Exe')],
  'C:\\Program Files': [D('Steam', 'Dir')],
  'C:\\Program Files\\Steam': [D('steam.exe', 'Exe')],
  'D:\\': [D('Jogos', 'Dir'), D('Coleção', 'Dir'), D('Truncada', 'Dir'), D('setup-antigo.exe', 'Exe'), D('na\u202Eme\u0007.exe', 'Exe')],
  'D:\\Jogos': [D('Aventura.exe', 'Exe'), D('Bruma.exe', 'Exe'), D('leiame.exe', 'Exe')],
  'E:\\': [],
  'D:\\Coleção': Array.from({ length: 60 }, (_, i) => (i % 5 === 0 ? D(`Pasta ${i / 5 + 1}`, 'Dir') : D(`Jogo ${i}.exe`, 'Exe'))),
  'D:\\Truncada': Array.from({ length: 2000 }, (_, i) => (i < 20 ? D(`Pasta ${String(i).padStart(4, '0')}`, 'Dir') : D(`Arquivo ${String(i).padStart(4, '0')}.exe`, 'Exe'))),
};
const BLOCKED_DIRS: Record<string, 'AccessDenied' | 'NotFound' | 'PathNotAllowed'> = {
  'C:\\Users': 'AccessDenied',
  'C:\\Program Files\\Steam\\sumiu': 'NotFound',
  '\\\\servidor\\jogos': 'PathNotAllowed',
};

let installed: Promise<void> | null = null;

/** Idempotente: o StrictMode do dev roda o efeito duas vezes e um segundo `mockIPC` zeraria os ouvintes. */
export function installDevCore(): Promise<void> {
  installed ??= install();
  return installed;
}

const reject = (code: string, params?: Record<string, unknown>) => Promise.reject({ code, params });

async function install(): Promise<void> {
  const params = new URLSearchParams(window.location.hash.replace(/^#/, '').replace(/^gallery&?/, ''));
  const lib = params.get('lib') ?? 'cheia';
  let games = makeGames(lib);
  let hidden: GameDto[] = params.get('hidden') === 'nenhum' ? [] : makeGames(params.get('hidden') === 'muitos' ? 'muitos' : 'cheia').slice(0, params.get('hidden') === 'muitos' ? 30 : 4).map((g, i) => ({ ...g, id: `epic:Oculto${i}`, name: `Jogo oculto ${i + 1}`, origin: ORIGINS[i % 4]! }));
  const snapshot = (focusGameId: string | null = null) => ({ games, focusGameId });
  const emitLibrary = (focusGameId: string | null = null) => void emitCoreEvent('library://changed', snapshot(focusGameId));

  // ---- sessao ----
  let session: SessionDto = { state: 'Idle', gameId: null, since: null, dismissed: false, elevated: false, minimized: false, launchFailure: null };
  const setSession = (patch: Partial<SessionDto>) => {
    session = { ...session, ...patch, since: Date.now() };
    void emitCoreEvent('session://state', session);
  };
  const closeMs = params.get('fast') === '1' ? 3000 : 10_000;
  let pendingReplace: string | null = null;
  let closeTimer: ReturnType<typeof setTimeout> | undefined;
  const markInProgress = (gameId: string | null) => {
    games = games.map((g) => ({ ...g, inProgress: g.id === gameId }));
  };
  const handlers: CoreHandlers = {};
  /** O jogo fechou: Idle, biblioteca com o foco no jogo e, na troca (CT-01), abre o proximo. */
  const gameClosed = () => {
    const id = session.gameId;
    markInProgress(null);
    setSession({ state: 'Idle', gameId: null, dismissed: false, minimized: false });
    emitLibrary(id);
    const next = pendingReplace;
    pendingReplace = null;
    if (next) setTimeout(() => void (handlers.session_launch as (a: Record<string, unknown>) => unknown)({ gameId: next }), 300);
  };
  const startClose = () => {
    setSession({ state: 'Closing', dismissed: false });
    clearTimeout(closeTimer);
    const stuck = params.get('trava') === 'yes';
    closeTimer = setTimeout(
      () => {
        if (stuck) {
          pendingReplace = null; // CT-01: a troca pendente se perde em CloseOverdue
          setSession({ state: 'CloseOverdue', dismissed: false });
        } else gameClosed();
      },
      stuck ? closeMs : 2200,
    );
  };
  const startRunning = (gameId: string) => {
    markInProgress(gameId);
    games = games.map((g) => (g.id === gameId ? { ...g, lastPlayedAt: Date.now() } : g));
    emitLibrary();
    setSession({ state: 'Running', gameId, dismissed: false, elevated: false, launchFailure: null });
  };
  const panel = (outcome: FrontOutcome) => {
    if (outcome === 'FrontAfterMinimize') session = { ...session, minimized: true };
    void emitCoreEvent('session://state', session);
    void emitCoreEvent('session://panel', { open: true, outcome });
  };

  // ---- volume e energia ----
  let volume: VolumeState = { level: 45, muted: false, available: params.get('audio') !== 'none' };
  const emitVolume = () => void emitCoreEvent('volume://changed', volume);
  const power: PowerCapabilities = {
    shutdown: 'Available',
    restart: 'Available',
    suspend: params.get('suspend') === 'no' ? 'Unavailable' : 'Available',
  };
  let ticket: { value: string; action: string } | null = null;

  // ---- configuracoes ----
  const monitorsFor = (): MonitorDto[] => {
    const all: MonitorDto[] = [
      { id: 'm1', name: 'Monitor Dell U2720Q', primary: true, current: false },
      { id: 'm2', name: 'TV Sala (HDMI)', primary: false, current: false },
    ];
    const list = params.get('monitors') === 'um' || params.get('monitors') === 'ausente' ? all.slice(0, 1) : all;
    const chosen = settings.monitorId && list.some((m) => m.id === settings.monitorId) ? settings.monitorId : list.find((m) => m.primary)!.id;
    return list.map((m) => ({ ...m, current: m.id === chosen }));
  };
  let settings: SettingsDto = {
    autostart: params.get('autostart') === 'on',
    autostartMechanism: params.get('autostart') === 'on' ? 'Task' : 'None',
    monitorId: params.get('monitors') === 'ausente' ? 'm2' : params.get('monitors') === 'um' ? null : 'm2',
    logDir: 'C:\\Users\\Sala\\AppData\\Local\\app.sofa.launcher\\logs',
  };

  const listDir = (path: string | null): DirListing | Promise<never> => {
    if (path === null || path === '') {
      if (params.get('fs') === 'vazio') return { path: null, parent: null, entries: [], truncated: false };
      return { path: null, parent: null, entries: [D('C:\\', 'Drive'), D('D:\\', 'Drive'), D('E:\\', 'Drive')], truncated: false };
    }
    const blocked = BLOCKED_DIRS[path];
    if (blocked) return reject(blocked);
    const entries = FS[path];
    if (!entries) return reject('NotFound');
    const isRoot = /^[A-Z]:\\$/.test(path);
    const parent = isRoot ? null : path.slice(0, path.lastIndexOf('\\')).replace(/^([A-Z]:)$/, '$1\\');
    return { path, parent, entries, truncated: path === 'D:\\Truncada' };
  };

  Object.assign(handlers, {
    library_get: () => snapshot(),
    library_refresh: () => {
      setTimeout(() => emitLibrary(), 1200);
    },
    library_hidden: () => hidden,
    game_unhide: ({ gameId }) => {
      const back = hidden.find((g) => g.id === gameId);
      if (!back) return reject('UnknownGame');
      hidden = hidden.filter((g) => g.id !== gameId);
      games = [...games, back];
      emitLibrary();
      return undefined;
    },
    library_add_manual: ({ path }) => {
      const p = String(path);
      if (/bruma\.exe$/i.test(p)) return reject('Duplicate');
      if (/leiame\.exe$/i.test(p)) return reject('NotAnExecutable');
      if (p.startsWith('\\\\')) return reject('PathNotAllowed');
      const file = p.slice(p.lastIndexOf('\\') + 1).replace(/\.exe$/i, '');
      const added: GameDto = { id: `manual:${Math.abs(hash(p)).toString(16).padStart(16, '0')}`, name: file, originalName: null, origin: 'Manual', cover: '', lastPlayedAt: null, inProgress: false };
      games = [...games, added];
      emitLibrary();
      return added;
    },
    fs_list_dir: ({ path }) => listDir(path as string | null),

    session_get: () => session,
    session_launch: ({ gameId }) => {
      const game = games.find((g) => g.id === gameId);
      if (session.state !== 'Idle') return reject('SessionAlreadyActive', { gameId: session.gameId });
      if (game?.name === 'Minério') return reject('NetworkPathBlocked');
      if (game?.name === 'Bruma') return reject('GameFileMissing', { gameId });
      if (game?.name === 'Guarda-Chuva Noturno') {
        setSession({ state: 'NeedsElevation', gameId: gameId as string, dismissed: false, elevated: false, launchFailure: null });
        return reject('NeedsElevation', { gameId });
      }
      setSession({ state: 'Launching', gameId: gameId as string, dismissed: false, elevated: false, launchFailure: null });
      if (game?.name !== 'Última Parada') setTimeout(() => startRunning(gameId as string), 6000);
      else setTimeout(() => setSession({ state: 'NotConfirmed' }), 4000);
      return undefined;
    },
    session_launch_with_uac: () => setSession({ state: 'Launching', elevated: true }),
    session_replace: ({ gameId }) => {
      if (session.state !== 'Running') return reject('InvalidState');
      pendingReplace = gameId as string;
      startClose();
      return undefined;
    },
    session_return_to_game: () => {
      if (session.state === 'Idle') return reject('NoActiveSession');
      if (session.state !== 'Running') return reject('InvalidState');
      if (params.get('fg') === 'denied') return reject('ForegroundDenied');
      return undefined;
    },
    session_close: () => {
      if (session.state === 'Idle') return reject('NoActiveSession');
      if (session.state !== 'Running') return reject('InvalidState');
      startClose();
      return undefined;
    },
    session_force_close: () => {
      if (session.state === 'Idle') return reject('NoActiveSession');
      if (session.state !== 'CloseOverdue') return reject('NotOverdue');
      if (params.get('force') === 'denied') return reject('AccessDenied');
      gameClosed();
      return undefined;
    },
    session_dismiss: () => {
      if (session.state === 'Launching' || session.state === 'Closing') setSession({ dismissed: true });
      else if (session.state === 'CloseOverdue') {
        clearTimeout(closeTimer);
        setSession({ state: 'Running', dismissed: false });
      } else if (session.state === 'NotConfirmed' || session.state === 'NeedsElevation') {
        setSession({ state: 'Idle', gameId: null, launchFailure: null });
      }
    },
    session_retry: () => setSession({ state: 'Launching', dismissed: false }),
    game_hide: ({ gameId }) => {
      const gone = games.find((g) => g.id === gameId);
      if (gone) hidden = [...hidden, gone];
      games = games.filter((g) => g.id !== gameId);
      emitLibrary();
    },
    game_rename: ({ gameId, name }) => {
      const trimmed = String(name).trim();
      if (!trimmed) return reject('EmptyName');
      games = games.map((g) => (g.id === gameId ? { ...g, name: trimmed, originalName: g.originalName ?? g.name } : g));
      emitLibrary();
      return games.find((g) => g.id === gameId);
    },
    game_restore_name: ({ gameId }) => {
      games = games.map((g) => (g.id === gameId && g.originalName ? { ...g, name: g.originalName, originalName: null } : g));
      emitLibrary();
      return games.find((g) => g.id === gameId);
    },
    input_status: () => ({ connected: params.get('ctrl') === 'none' ? 0 : 1 }),

    volume_get: () => volume,
    volume_step: ({ direction }) => {
      if (!volume.available) return reject('NoAudioDevice');
      // CA-40: RT no mudo desmuta e sobe; LT no mudo baixa o nivel guardado e mantem o mudo (CA-64).
      const delta = direction === 'Up' ? 5 : -5;
      volume = { ...volume, level: Math.max(0, Math.min(100, volume.level + delta)), muted: direction === 'Up' ? false : volume.muted };
      emitVolume();
      return volume;
    },
    volume_toggle_mute: () => {
      if (!volume.available) return reject('NoAudioDevice');
      volume = { ...volume, muted: !volume.muted };
      emitVolume();
      return volume;
    },
    power_capabilities: () => power,
    power_prepare: ({ action }) => {
      const key = String(action).toLowerCase() as keyof PowerCapabilities;
      if (power[key] === 'Unavailable') return reject('Unavailable');
      ticket = { value: `T${Math.random().toString(36).slice(2).padEnd(22, 'x')}`, action: String(action) };
      return ticket.value;
    },
    power_execute: ({ ticket: given }) => {
      if (!ticket || ticket.value !== given) return reject('TicketInvalid');
      const used = ticket;
      ticket = null;
      if (params.get('power') === 'fail') return reject('Rejected');
      // O PC "acorda" dois segundos depois de suspender, para ver a UI voltar a mesma tela (CA-46).
      if (used.action === 'Suspend') setTimeout(() => void emitCoreEvent('system://resumed', {}), 2500);
      return undefined;
    },
    app_exit: () => undefined,

    settings_get: () => settings,
    settings_set_autostart: ({ enabled }) => {
      if (params.get('autostart') === 'fail') return reject('AutostartFailed');
      settings = { ...settings, autostart: Boolean(enabled), autostartMechanism: enabled ? 'Task' : 'None' };
      return settings;
    },
    settings_set_monitor: ({ monitorId }) => {
      if (monitorId !== null && !monitorsFor().some((m) => m.id === monitorId)) return reject('UnknownMonitor');
      settings = { ...settings, monitorId: monitorId as string | null };
      return settings;
    },
    display_monitors: () => monitorsFor(),
  } satisfies CoreHandlers);
  mockCore(handlers);

  // Sessao ja em andamento, para ver o painel e o fechamento sem esperar a abertura.
  const start = params.get('start');
  if (start === 'running' || start === 'panel') {
    const first = games[0];
    if (first) {
      startRunning(first.id);
      if (start === 'panel') setTimeout(() => panel('Front'), 700);
    }
  }
  // F2 simula a combinacao de retorno (no produto ela vem do nucleo, com o Sofa em segundo plano).
  window.addEventListener('keydown', (e) => {
    if (e.key === 'F2' && session.state === 'Running') panel(params.get('minimized') === '1' ? 'FrontAfterMinimize' : 'Front');
  });
  Object.assign(window, {
    __devCore: {
      panel,
      resume: () => void emitCoreEvent('system://resumed', {}),
      displayChanged: () => void emitCoreEvent('display://changed', monitorsFor()),
      volume: (v: Partial<VolumeState>) => {
        volume = { ...volume, ...v };
        emitVolume();
      },
    },
  });

  // Roteiro de controle para conferir o aviso (CA-14, CA-15): `#ctrl=lost`.
  if (params.get('ctrl') === 'lost') {
    setTimeout(() => void emitCoreEvent('input://controllers', { connected: 0 }), 1500);
    setTimeout(() => void emitCoreEvent('input://controllers', { connected: 1 }), 6000);
  }
}

function hash(text: string): number {
  let h = 0;
  for (let i = 0; i < text.length; i++) h = (Math.imul(31, h) + text.charCodeAt(i)) | 0;
  return h;
}
