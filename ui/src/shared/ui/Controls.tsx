/** Controles so visuais (o foco e da linha que os contem): chave liga/desliga e barra de nivel. */
export function Switch({ on }: { on: boolean }) {
  return <span className={`sw${on ? ' on' : ''}`} aria-hidden="true" />;
}

/** Barra de 0 a 100. `muted` pinta de cinza (volume no mudo). */
export function Slider({ value, muted = false }: { value: number; muted?: boolean }) {
  const pct = Math.max(0, Math.min(100, value));
  return (
    <div className={`slider${muted ? ' m' : ''}`} aria-hidden="true">
      <i style={{ width: `${pct}%` }} />
    </div>
  );
}
