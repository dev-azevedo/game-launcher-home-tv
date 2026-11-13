import tokens from '@brand/tokens.json';

/** Valores de movimento da marca para uso em JS (arch 3.5). O CSS usa os mesmos pelos tokens.css. */
export const motion = {
  fastMs: tokens.motion.duration.fast,
  baseMs: tokens.motion.duration.base,
  slowMs: tokens.motion.duration.slow,
  easeOut: tokens.motion.easing.out,
  easeInOut: tokens.motion.easing.inOut,
} as const;

export const focus = {
  scale: tokens.focus.scale,
  dimOthers: tokens.focus.dimOthers,
} as const;

/** Tempos de produto (handoff 10, CA-03, RN-31). */
export const timing = {
  cursorHideMs: 3000,
  waitScreenGuardMs: 600,
  volumeHudMs: 2000,
} as const;
