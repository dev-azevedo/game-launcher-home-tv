import { create } from 'zustand';
import { errorOf, events, ipc, type VolumeDirection, type VolumeState } from '@/shared/ipc';
import { timing } from '@/shared/theme/tokens';

interface VolumeStore {
  /** Estado do volume mestre do Windows; `null` ate a primeira leitura. */
  volume: VolumeState | null;
  /** Indicador visivel (LT/RT na biblioteca, CA-64). */
  hud: boolean;
  apply: (volume: VolumeState) => void;
  load: () => Promise<void>;
  /** `hud`: LT/RT mostram o indicador por 2 s; no menu rapido o nivel ja esta na tela. */
  step: (direction: VolumeDirection, options?: { hud?: boolean }) => Promise<void>;
  toggleMute: () => Promise<void>;
  hideHud: () => void;
}

let hudTimer: ReturnType<typeof setTimeout> | undefined;

/** Sem dispositivo (`NoAudioDevice`): o nivel nao importa; marca indisponivel (CA-42). */
const UNAVAILABLE: VolumeState = { level: 0, muted: false, available: false };

export const useVolume = create<VolumeStore>((set) => {
  const showHud = () => {
    set({ hud: true });
    clearTimeout(hudTimer);
    // Some 2 s depois do ultimo uso (CA-64): cada puxada reinicia a contagem.
    hudTimer = setTimeout(() => set({ hud: false }), timing.volumeHudMs);
  };
  const run = async (call: () => Promise<VolumeState>, hud: boolean) => {
    try {
      set({ volume: await call() });
    } catch (thrown) {
      if (errorOf(thrown).code === 'NoAudioDevice') set({ volume: { ...UNAVAILABLE } });
    }
    if (hud) showHud();
  };
  return {
    volume: null,
    hud: false,
    apply: (volume) => set({ volume }),
    load: async () => {
      try {
        set({ volume: await ipc.volumeGet() });
      } catch {
        // Sem leitura o menu mostra o controle como indisponivel so depois de um erro de comando.
      }
    },
    step: (direction, { hud = false } = {}) => run(() => ipc.volumeStep(direction), hud),
    toggleMute: () => run(() => ipc.volumeToggleMute(), false),
    hideHud: () => {
      clearTimeout(hudTimer);
      set({ hud: false });
    },
  };
});

/** Carga inicial + `volume://changed` (mudanca interna, externa e troca de dispositivo, CA-41). */
export function startVolume(): () => void {
  const off = events.volumeChanged((volume) => useVolume.getState().apply(volume));
  void useVolume.getState().load();
  return () => {
    off();
    useVolume.getState().hideHud();
  };
}
