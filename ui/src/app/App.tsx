import { lazy, Suspense, useEffect, useMemo, useState } from 'react';
import { BootSplash } from './BootSplash';
import { CursorAutoHide } from './CursorAutoHide';
import { InputBridge } from './InputBridge';
import { ScreenStack } from './ScreenStack';
import { startSpatialNavigation } from './navigation/spatial';
import { startLibrary, useLibrary } from '@/features/library/store';
import { startSession } from '@/features/session/store';
import { createKeyboardSource } from '@/shared/input/keyboard';
import { mergeSources } from '@/shared/input/intents';
import { createCoreIntentSource } from '@/shared/ipc';

startSpatialNavigation();

// Galeria e nucleo simulado so existem no dev server: saem do build de producao.
const Gallery = import.meta.env.DEV ? lazy(() => import('../dev/Gallery')) : null;
const needsDevCore = () => import.meta.env.DEV && !('__TAURI_INTERNALS__' in window);

export function App() {
  // Uma rota so: nucleo (`input://intent`) + teclado de emergencia (RN-07).
  const source = useMemo(() => mergeSources(createCoreIntentSource(), createKeyboardSource(window)), []);
  const showGallery = Gallery !== null && window.location.hash.startsWith('#gallery');
  const libraryStatus = useLibrary((s) => s.status);
  const [coreReady, setCoreReady] = useState(!needsDevCore());

  useEffect(() => {
    let cancelled = false;
    let stops: Array<() => void> = [];
    void (async () => {
      if (import.meta.env.DEV && needsDevCore()) await (await import('../dev/devCore')).installDevCore();
      if (cancelled) return;
      setCoreReady(true);
      stops = [startLibrary(), startSession()];
    })();
    return () => {
      cancelled = true;
      stops.forEach((stop) => stop());
    };
  }, []);

  if (!coreReady) return null;
  return (
    <>
      <InputBridge source={source} />
      <CursorAutoHide />
      {showGallery && Gallery ? (
        <Suspense fallback={null}>
          <Gallery />
        </Suspense>
      ) : (
        <>
          <ScreenStack />
          {libraryStatus === 'loading' ? <BootSplash /> : null}
        </>
      )}
    </>
  );
}
