import { Focusable, FocusLayer } from '@/shared/ui/Focusable';
import { Icon } from '@/shared/ui/Icon';
import { FOCUS_ADD, FOCUS_REFRESH } from '../domain';

interface EmptyLibraryProps {
  active: boolean;
  onAdd: () => void;
  onRefresh: () => void;
  refreshing: boolean;
}

/** Biblioteca vazia (CA-23): "Adicionar jogo" ja em foco, "Atualizar biblioteca" ao lado. */
export function EmptyLibrary({ active, onAdd, onRefresh, refreshing }: EmptyLibraryProps) {
  return (
    <FocusLayer focusKey="home-empty" defaultFocus={FOCUS_ADD} active={active} rememberFocus>
      <div className="empty">
        <div className="big">Nenhum jogo por aqui</div>
        <p>Instale jogos na Steam, na Epic ou no GOG, ou adicione um executável.</p>
        <div className="btnrow">
          <Focusable as="button" focusKey={FOCUS_ADD} className="btn primary" onActivate={onAdd}>
            <Icon name="plus" />
            Adicionar jogo
          </Focusable>
          <Focusable as="button" focusKey={FOCUS_REFRESH} className="btn" onActivate={onRefresh} disabled={refreshing}>
            <Icon name="refresh" />
            Atualizar biblioteca
          </Focusable>
        </div>
      </div>
    </FocusLayer>
  );
}

/** Primeira carga sem cache: esqueleto de cards, sem bloquear o menu (handoff 5). */
export function LoadingLibrary({ active, onAdd }: { active: boolean; onAdd: () => void }) {
  return (
    <FocusLayer focusKey="home-loading" defaultFocus={FOCUS_ADD} active={active}>
      <div className="empty" role="status">
        <div className="big">Procurando jogos…</div>
        <p>Lendo a Steam, a Epic e o GOG deste PC.</p>
        <div className="skeletons" aria-hidden="true">
          {[0, 1, 2, 3, 4, 5].map((i) => (
            <div className="card" key={i}>
              <div className="poster sk" />
            </div>
          ))}
        </div>
        <div className="btnrow">
          <Focusable as="button" focusKey={FOCUS_ADD} className="btn primary" onActivate={onAdd}>
            <Icon name="plus" />
            Adicionar jogo
          </Focusable>
        </div>
      </div>
    </FocusLayer>
  );
}
