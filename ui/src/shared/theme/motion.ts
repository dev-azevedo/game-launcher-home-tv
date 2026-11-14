/** Movimento reduzido: preferencia do sistema ou o modo `html.reduced` (handoff 9, 10). */
export function prefersReducedMotion(): boolean {
  if (document.documentElement.classList.contains('reduced')) return true;
  return typeof window.matchMedia === 'function' && window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

export const scrollBehavior = (instant = false): ScrollBehavior =>
  instant || prefersReducedMotion() ? 'auto' : 'smooth';
