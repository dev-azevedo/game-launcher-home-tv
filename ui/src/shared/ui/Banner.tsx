import { Icon, type IconName } from './Icon';

export type BannerTone = 'warn' | 'lost' | 'ok';

const TONE_ICON: Record<BannerTone, IconName> = { warn: 'gamepad', lost: 'alert', ok: 'check' };

interface BannerProps {
  tone: BannerTone;
  children: string;
}

/** Faixa no topo (controle ausente, desconectado, ok). Nao toma foco (CA-13, CA-14). */
export function Banner({ tone, children }: BannerProps) {
  return (
    <div className="banner-slot">
      <div className={`banner ${tone === 'warn' ? '' : tone}`.trim()} role="status" aria-live="polite">
        <Icon name={TONE_ICON[tone]} />
        <span>{children}</span>
      </div>
    </div>
  );
}
