import { Icon } from './Icon';

export type Glyph = 'A' | 'B' | 'view' | 'menu' | 'LB' | 'RB' | 'LT' | 'RT' | 'dpad';

export interface HintSpec {
  glyphs: Glyph[];
  label: string;
}

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

/** Rodape de dicas com os botoes que valem na tela. Decorativo para leitor de tela. */
export function ButtonHints({ hints }: { hints: HintSpec[] }) {
  return (
    <div className="hints">
      {hints.map((hint) => (
        <span className="hint" key={hint.label}>
          {hint.glyphs.map((glyph) => (
            <GlyphMark key={glyph} glyph={glyph} />
          ))}
          <span>{hint.label}</span>
        </span>
      ))}
    </div>
  );
}
