import { useInputMode } from '@/shared/input/mode';
import { Icon } from './Icon';

export type Glyph = 'A' | 'B' | 'view' | 'menu' | 'LB' | 'RB' | 'LT' | 'RT' | 'dpad';

export interface HintSpec {
  glyphs: Glyph[];
  label: string;
}

/** Tecla do teclado de emergencia para cada botao do controle (shared/input/keyboard.ts, RN-07). */
export const KEY_FOR_GLYPH: Record<Glyph, string> = {
  A: 'Enter',
  B: 'Esc',
  view: 'F2',
  menu: 'F1',
  LB: 'PgUp',
  RB: 'PgDn',
  LT: '[',
  RT: ']',
  dpad: 'Setas',
};

function GlyphMark({ glyph }: { glyph: Glyph }) {
  if (glyph === 'view' || glyph === 'menu') {
    return (
      <span className="g" aria-hidden="true">
        <Icon name={glyph} />
      </span>
    );
  }
  if (glyph === 'A' || glyph === 'B') {
    return (
      <span className="g" aria-hidden="true">
        {glyph}
      </span>
    );
  }
  return (
    <span className="g pill" aria-hidden="true">
      {glyph === 'dpad' ? 'D-pad' : glyph}
    </span>
  );
}

function KeyMark({ glyph }: { glyph: Glyph }) {
  return (
    <kbd className="g key" aria-hidden="true">
      {KEY_FOR_GLYPH[glyph]}
    </kbd>
  );
}

const ARROWS_HINT: HintSpec = { glyphs: ['dpad'], label: 'Mover' };

/**
 * Rodape de dicas com os botoes que valem na tela. Decorativo para leitor de tela. Usando o teclado,
 * mostra as teclas (Enter, Esc, F1...) e acrescenta "Setas · Mover".
 */
export function ButtonHints({ hints }: { hints: HintSpec[] }) {
  const keyboard = useInputMode((s) => s.mode === 'keyboard');
  const shown = keyboard && !hints.some((h) => h.glyphs.includes('dpad')) ? [ARROWS_HINT, ...hints] : hints;
  return (
    <div className="hints" data-input-mode={keyboard ? 'keyboard' : 'gamepad'}>
      {shown.map((hint) => (
        <span className="hint" key={hint.label}>
          {hint.glyphs.map((glyph) =>
            keyboard ? <KeyMark key={glyph} glyph={glyph} /> : <GlyphMark key={glyph} glyph={glyph} />,
          )}
          <span>{hint.label}</span>
        </span>
      ))}
    </div>
  );
}
