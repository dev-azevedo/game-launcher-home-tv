import type { ReactNode } from 'react';

/** Icones do mockup (trace de 24x24). SVG estatico em JSX: nenhum HTML montado por string (SEC-07). */
const PATHS = {
  play: <path d="M8 5.5v13l11-6.5z" fill="currentColor" />,
  gamepad: (
    <>
      <path d="M7.5 7.5h9a4.5 4.5 0 014.4 5.4l-.7 3.3a2.6 2.6 0 01-4.4 1.3L14 15.5h-4l-1.8 2a2.6 2.6 0 01-4.4-1.3l-.7-3.3a4.5 4.5 0 014.4-5.4z" />
      <path d="M8 10.5v3M6.5 12h3" />
      <circle cx="15.5" cy="11" r=".6" />
      <circle cx="17.5" cy="13" r=".6" />
    </>
  ),
  power: (
    <>
      <path d="M12 3.5v8" />
      <path d="M6.8 6.9a7.5 7.5 0 1010.4 0" />
    </>
  ),
  eyeoff: (
    <>
      <path d="M3.5 3.5l17 17" />
      <path d="M10 6.2A8.6 8.6 0 0112 6c4 0 7 3.2 9 6a15 15 0 01-3 3.4M6.4 7.4C4.5 8.7 3 10.5 2.5 12c1.8 3 5 6 9.5 6 1.2 0 2.3-.2 3.3-.6" />
      <path d="M9.9 10a3 3 0 004.1 4.1" />
    </>
  ),
  exe: (
    <>
      <rect x="4" y="4" width="16" height="16" rx="2" />
      <path d="M4 9h16" />
      <path d="M8 13l2 2-2 2M12 17h4" />
    </>
  ),
  check: <path d="M5 12.5l4.5 4.5L19 7.5" />,
  alert: (
    <>
      <path d="M12 4l9 16H3L12 4z" />
      <path d="M12 10v4.5M12 17.5h.01" />
    </>
  ),
  chevron: <path d="M9 6l6 6-6 6" />,
  grid: (
    <>
      <rect x="4" y="4" width="6.5" height="6.5" rx="1" />
      <rect x="13.5" y="4" width="6.5" height="6.5" rx="1" />
      <rect x="4" y="13.5" width="6.5" height="6.5" rx="1" />
      <rect x="13.5" y="13.5" width="6.5" height="6.5" rx="1" />
    </>
  ),
  pencil: <path d="M4 20l1-4L16 5a2.1 2.1 0 013 3L8 19l-4 1z" />,
  refresh: (
    <>
      <path d="M19.5 12a7.5 7.5 0 11-2.4-5.5" />
      <path d="M19.5 4.5v4.5H15" />
    </>
  ),
} satisfies Record<string, ReactNode>;

export type IconName = keyof typeof PATHS;
export type IconSize = 's' | 'm' | 'l' | 'xl';

export function Icon({ name, size = 'm' }: { name: IconName; size?: IconSize }) {
  return (
    <svg
      className={size === 'm' ? 'ic' : `ic ${size}`}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      {PATHS[name]}
    </svg>
  );
}
