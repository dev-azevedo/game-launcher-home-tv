import { selectTop, useNavigation } from '@/app/navigation/store';
import { Slider } from '@/shared/ui/Controls';
import { Icon } from '@/shared/ui/Icon';
import { hudLabel } from './domain';
import { useVolume } from './store';

/**
 * Indicador de volume (CA-64): pilula no canto superior direito, aparece em cada puxada de LT/RT e some
 * 2 s depois. So na biblioteca (Home e Todos os jogos); menu, dialogo e painel nao mostram (CA-65).
 */
export function VolumeHud() {
  const visible = useVolume((s) => s.hud);
  const volume = useVolume((s) => s.volume);
  const onLibrary = useNavigation((s) => ['home', 'allGames'].includes(selectTop(s).kind));
  if (!visible || !volume || !onLibrary) return null;
  const unavailable = !volume.available;
  return (
    <div className="hud-slot">
      <div className={`vol${volume.muted || unavailable ? ' muted' : ''}`} role="status" aria-live="polite">
        <Icon name={volume.muted || unavailable ? 'mute' : 'vol'} />
        {unavailable ? null : <Slider value={volume.level} muted={volume.muted} />}
        <span className="lbl">{hudLabel(volume)}</span>
      </div>
    </div>
  );
}
