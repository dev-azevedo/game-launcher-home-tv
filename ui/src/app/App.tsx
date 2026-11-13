import { lazy, Suspense, useMemo } from 'react';
import { BootSplash } from './BootSplash';
import { CursorAutoHide } from './CursorAutoHide';
import { InputBridge } from './InputBridge';
import { startSpatialNavigation } from './navigation/spatial';
import { createKeyboardSource } from '@/shared/input/keyboard';

startSpatialNavigation();

// Galeria so existe no dev server: e removida do build de producao.
const Gallery = import.meta.env.DEV ? lazy(() => import('../dev/Gallery')) : null;

export function App() {
  // Fonte do nucleo (`input://intent`) entra em shared/ipc quando o contrato sair (FE-02).
  const source = useMemo(() => createKeyboardSource(window), []);
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
