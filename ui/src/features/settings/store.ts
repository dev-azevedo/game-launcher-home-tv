import { create } from 'zustand';
import { useLibrary } from '@/features/library/store';
import { errorCopyFor } from '@/shared/domain/errors';
import { errorOf, ipc, type GameDto, type GameId, type MonitorDto, type SettingsDto } from '@/shared/ipc';

interface SettingsStore {
  settings: SettingsDto | null;
  monitors: MonitorDto[];
  hidden: GameDto[];
  loaded: boolean;
  autostartFailed: boolean;
  /** Evita duas chamadas de autostart ao mesmo tempo (o nucleo recusa `Busy`-like pelo registro). */
  savingAutostart: boolean;
  load: () => Promise<void>;
  setMonitors: (monitors: MonitorDto[]) => void;
  setAutostart: (enabled: boolean) => Promise<void>;
  chooseMonitor: (monitor: MonitorDto) => Promise<void>;
  restore: (gameId: GameId) => Promise<void>;
}

export const useSettings = create<SettingsStore>((set, get) => ({
  settings: null,
  monitors: [],
  hidden: [],
  loaded: false,
  autostartFailed: false,
  savingAutostart: false,
  // Tres leituras independentes: uma falha nao esconde as outras abas.
  load: async () => {
    const [settings, monitors, hidden] = await Promise.allSettled([
      ipc.settingsGet(),
      ipc.displayMonitors(),
      ipc.libraryHidden(),
    ]);
    set((s) => ({
      loaded: true,
      settings: settings.status === 'fulfilled' ? settings.value : s.settings,
      monitors: monitors.status === 'fulfilled' ? monitors.value : s.monitors,
      hidden: hidden.status === 'fulfilled' ? hidden.value : s.hidden,
    }));
  },
  setMonitors: (monitors) => set({ monitors }),
  setAutostart: async (enabled) => {
    if (get().savingAutostart) return;
    set({ savingAutostart: true, autostartFailed: false });
    try {
      set({ settings: await ipc.settingsSetAutostart(enabled) });
    } catch {
      // `AutostartFailed` (inclui exe fora de %ProgramFiles%): linha de erro, o item segue em foco.
      set({ autostartFailed: true });
    } finally {
      set({ savingAutostart: false });
    }
  },
  chooseMonitor: async (monitor) => {
    const notice = useLibrary.getState().setNotice;
    try {
      set({ settings: await ipc.settingsSetMonitor(monitor.id) });
      notice(`SofaLauncher movido para ${monitor.name}.`);
    } catch (thrown) {
      notice(errorCopyFor(errorOf(thrown)).message);
      void get().load();
    }
  },
  restore: async (gameId) => {
    const name = get().hidden.find((g) => g.id === gameId)?.name ?? 'O jogo';
    const notice = useLibrary.getState().setNotice;
    try {
      await ipc.gameUnhide(gameId);
      notice(`${name} voltou para a biblioteca.`);
    } catch (thrown) {
      notice(errorCopyFor(errorOf(thrown)).message);
    }
    try {
      set({ hidden: await ipc.libraryHidden() });
    } catch {
      set((s) => ({ hidden: s.hidden.filter((g) => g.id !== gameId) }));
    }
  },
}));
