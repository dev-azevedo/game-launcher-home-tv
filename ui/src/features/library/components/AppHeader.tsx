import type { ReactNode } from 'react';
import { BrandLogo } from '@/shared/ui/Brand';
import { Icon } from '@/shared/ui/Icon';

interface AppHeaderProps {
  /** Chip "Atualizando biblioteca…" (handoff 5). */
  refreshing?: boolean;
  /** Chips extras (controle, volume). */
  children?: ReactNode;
}

/** Cabecalho das telas de biblioteca: marca a esquerda, indicadores a direita. */
export function AppHeader({ refreshing = false, children }: AppHeaderProps) {
  return (
    <div className="hdr">
      <BrandLogo />
      <div className="chips">
        {refreshing ? (
          <span className="chip busy" role="status">
            <Icon name="refresh" size="s" />
            <span>Atualizando biblioteca…</span>
          </span>
        ) : null}
        {children}
      </div>
    </div>
  );
}
