import { lazy, Suspense, useMemo } from 'react';
import { BootSplash } from './BootSplash';
import { CursorAutoHide } from './CursorAutoHide';
import { InputBridge } from './InputBridge';
import { startSpatialNavigation } from './navigation/spatial';
import { createKeyboardSource } from '@/shared/input/keyboard';
import { mergeSources } from '@/shared/input/intents';
import { createCoreIntentSource } from '@/shared/ipc';

startSpatialNavigation();

// Galeria so existe no dev server: e removida do build de producao.
const Gallery = import.meta.env.DEV ? lazy(() => import('../dev/Gallery')) : null;

export function App() {
  // Uma rota so: nucleo (`input://intent`) + teclado de emergencia (RN-07).
  const source = useMemo(() => mergeSources(createCoreIntentSource(), createKeyboardSource(window)), []);
  const showGallery = Gallery !== null && window.location.hash === '#gallery';
  return (
    <>
      <InputBridge source={source} />
      <CursorAutoHide />
      {showGallery && Gallery ? (
        <Suspense fallback={null}>
          <Gallery />
        </Suspense>
      ) : (
        <BootSplash />
      )}
    </>
  );
}
