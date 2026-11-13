import { useEffect } from 'react';
import { timing } from '@/shared/theme/tokens';

/** CA-03: o cursor some apos 3 s parado e volta ao mexer o mouse. */
export function CursorAutoHide({ delayMs = timing.cursorHideMs }: { delayMs?: number }) {
  useEffect(() => {
    const root = document.documentElement;
    let timer: ReturnType<typeof setTimeout>;
    const hide = () => root.classList.add('nocursor');
    const onMove = () => {
      root.classList.remove('nocursor');
      clearTimeout(timer);
      timer = setTimeout(hide, delayMs);
    };
    timer = setTimeout(hide, delayMs);
    window.addEventListener('mousemove', onMove);
    return () => {
      clearTimeout(timer);
      window.removeEventListener('mousemove', onMove);
      root.classList.remove('nocursor');
    };
  }, [delayMs]);
  return null;
}
