import type { VolumeState } from '@/shared/ipc';

/** Texto do indicador: numero de 0 a 100, "Mudo" ou "Sem áudio" (microcopy, Geral). */
export function hudLabel(volume: VolumeState): string {
  if (!volume.available) return 'Sem áudio';
  if (volume.muted) return 'Mudo';
  return String(volume.level);
}
