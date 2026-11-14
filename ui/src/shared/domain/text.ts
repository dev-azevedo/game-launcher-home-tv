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

// eslint-disable-next-line no-control-regex
const HIDDEN_CHARS = /[\u0000-\u001F\u007F-\u009F\u061C\u200E\u200F\u202A-\u202E\u2066-\u2069]/gu;

/**
 * Nome cru de arquivo para EXIBICAO (CT-03, SEC-07): controle (C0/C1) e bidi viram um simbolo visivel
 * em vez de agir (inverter o texto, apagar o fim). So para mostrar: a navegacao usa o nome original.
 */
export function neutralizeName(raw: string): string {
  return raw.replace(HIDDEN_CHARS, (ch) => {
    const code = ch.codePointAt(0) ?? 0;
    if (code <= 0x1f) return String.fromCodePoint(0x2400 + code); // simbolos de controle (␇)
    if (code === 0x7f) return '\u2421';
    return '\uFFFD';
  });
}
