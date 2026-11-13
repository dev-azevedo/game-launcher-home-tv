/**
 * Truncamento de nome para exibicao (E-13). O CSS faz o corte visual com reticencias;
 * esta funcao serve onde o texto precisa de limite em caracteres (ex.: aria-label, toast).
 * Conta pontos de codigo, nao unidades UTF-16, para nao partir emoji ao meio.
 */
export function truncateName(name: string, max: number): string {
  if (max <= 0) return '';
  const chars = Array.from(name);
  if (chars.length <= max) return name;
  return `${chars.slice(0, Math.max(0, max - 1)).join('').trimEnd()}…`;
}
