import type { IconName } from '@/shared/ui/Icon';

/**
 * Layouts da tela inicial. `sofa` e o padrao do app; os outros dois sao releituras de
 * dashboards de console (fileira de icones com arte em tela cheia, e mosaico de blocos).
 */
export type LayoutId = 'sofa' | 'vitrine' | 'mosaico';

export interface LayoutSpec {
  id: LayoutId;
  label: string;
  hint: string;
  icon: IconName;
}

export const DEFAULT_LAYOUT: LayoutId = 'sofa';

export const LAYOUTS: readonly LayoutSpec[] = [
  { id: 'sofa', label: 'Sofá (padrão)', hint: 'Destaque do jogo no alto e cartazes em duas fileiras.', icon: 'posters' },
  {
    id: 'vitrine',
    label: 'Vitrine',
    hint: 'Fileira de ícones no alto e a arte do jogo em tela cheia. Inspirado no PS5.',
    icon: 'strip',
  },
  {
    id: 'mosaico',
    label: 'Mosaico',
    hint: 'Blocos quadrados, o último jogado em destaque e acento verde. Inspirado no Xbox Series.',
    icon: 'tiles',
  },
];

/** Valor salvo ou qualquer coisa estranha vira o padrao. */
export function parseLayout(value: unknown): LayoutId {
  return LAYOUTS.some((l) => l.id === value) ? (value as LayoutId) : DEFAULT_LAYOUT;
}

export function layoutLabel(id: LayoutId): string {
  return LAYOUTS.find((l) => l.id === id)!.label;
}

/** Proximo/anterior na lista, com volta (menu rapido: ←/→). */
export function cycleLayout(id: LayoutId, delta: 1 | -1): LayoutId {
  const i = LAYOUTS.findIndex((l) => l.id === id);
  return LAYOUTS[(i + delta + LAYOUTS.length) % LAYOUTS.length]!.id;
}
