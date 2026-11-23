import { useEffect, useState, type ReactNode } from 'react';
import { useNavigation } from '@/app/navigation/store';
import type { LayoutId } from '@/features/appearance/domain';
import { Focusable } from '@/shared/ui/Focusable';
import { Icon } from '@/shared/ui/Icon';

export const FOCUS_HDR_SETTINGS = 'hdr-settings';

/** Hora local "HH:MM", atualizada a cada 15 s. */
function useClock(): string {
  const format = () => new Date().toLocaleTimeString('pt-BR', { hour: '2-digit', minute: '2-digit' });
  const [now, setNow] = useState(format);
  useEffect(() => {
    const id = window.setInterval(() => setNow(format()), 15000);
    return () => window.clearInterval(id);
  }, []);
  return now;
}

function Avatar() {
  return (
    <span className="c-avatar" aria-hidden="true">
      <Icon name="gamepad" size="s" />
    </span>
  );
}

interface ConsoleHeaderProps {
  layout: Exclude<LayoutId, 'sofa'>;
  refreshing?: boolean;
  /** Chips extras (controle). */
  children?: ReactNode;
}

/**
 * Barra do topo dos layouts de console. Vitrine: aba "Jogos" a esquerda; configuracoes, avatar e relogio a direita.
 * Mosaico: avatar a esquerda; configuracoes e relogio a direita. O botao de configuracoes recebe foco (↑ a partir
 * da fileira) e abre a tela de configuracoes.
 */
export function ConsoleHeader({ layout, refreshing = false, children }: ConsoleHeaderProps) {
  const clock = useClock();
  const push = useNavigation((s) => s.push);
  const settings = (
    <Focusable
      focusKey={FOCUS_HDR_SETTINGS}
      className="c-iconbtn"
      role="button"
      ariaLabel="Configurações"
      onActivate={() => push('settings', FOCUS_HDR_SETTINGS)}
    >
      <Icon name="sliders" size="s" />
    </Focusable>
  );
  return (
    <div className="hdr c-hdr" data-variant={layout}>
      <div className="c-left">
        {layout === 'vitrine' ? (
          <span className="c-tab sel" aria-current="page">
            Jogos
          </span>
        ) : (
          <Avatar />
        )}
      </div>
      <div className="chips">
        {refreshing ? (
          <span className="chip busy" role="status">
            <Icon name="refresh" size="s" />
            <span>Atualizando biblioteca…</span>
          </span>
        ) : null}
        {children}
        {settings}
        {layout === 'vitrine' ? <Avatar /> : null}
        <span className="c-clock" aria-label={`Agora são ${clock}`}>
          {clock}
        </span>
      </div>
    </div>
  );
}
