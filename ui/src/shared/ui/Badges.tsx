import type { Origin } from '@/shared/ipc/types';

const ORIGIN_LABEL: Record<Origin, string> = {
  Steam: 'Steam',
  Epic: 'Epic',
  Gog: 'GOG',
  Manual: 'Manual',
};

export function originLabel(origin: Origin): string {
  return ORIGIN_LABEL[origin];
}

/** Origem discreta no card (CA-60, RN-25). Fundo escuro para contraste sobre capas. */
export function OriginBadge({ origin }: { origin: Origin }) {
  return <span className="pill glass b-origin">{ORIGIN_LABEL[origin]}</span>;
}

/** Selo "Em andamento" (CA-36). */
export function InProgressBadge() {
  return <span className="pill amber b-prog">Em andamento</span>;
}
