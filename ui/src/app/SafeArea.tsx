import type { ReactNode } from 'react';

/**
 * Area segura de 5% (CA-54, `--safe-x`/`--safe-y` da marca). Texto e itens focaveis ficam dentro.
 * Unica excecao (RN-32): card de capa cortado na borda de uma fileira, que usa `.safe-bleed`
 * (margem negativa) dentro deste container.
 */
export function SafeArea({ children }: { children: ReactNode }) {
  return <div className="safe">{children}</div>;
}
