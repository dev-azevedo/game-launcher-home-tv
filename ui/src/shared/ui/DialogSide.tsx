import { CoverImage } from './Cover';
import { Icon, type IconName } from './Icon';
import type { DialogTone } from './ConfirmDialog';

/** Jogo cuja capa aparece no dialogo (so id, nome e se tem capa: nada de caminho, SEC-03). */
export interface DialogCover {
  id: string;
  name: string;
  hasCover: boolean;
}

interface DialogSideProps {
  /** Capa 112x168 (tem prioridade sobre o icone, como no mockup). */
  cover?: DialogCover | null;
  icon?: IconName;
  tone?: DialogTone;
}

/** Lado esquerdo dos dialogos: capa do jogo ou icone tematico (handoff, DES-BUG-01). */
export function DialogSide({ cover, icon = 'alert', tone = 'warn' }: DialogSideProps) {
  return (
    <div className="side">
      {cover ? (
        <div className="thumb">
          <CoverImage gameId={cover.hasCover ? cover.id : null} name={cover.name} />
        </div>
      ) : (
        <div className={`badge-ic${tone === 'warn' ? '' : ` ${tone}`}`}>
          <Icon name={icon} />
        </div>
      )}
    </div>
  );
}
