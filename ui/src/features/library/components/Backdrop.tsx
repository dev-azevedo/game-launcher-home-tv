import { useState } from 'react';
import { coverUrl, type GameDto } from '@/shared/ipc';

/**
 * Arte do jogo em foco em tela cheia, atras de tudo (layouts Vitrine e Mosaico). Usa a mesma capa do
 * cartaz (SEC-08). `layered`: uma copia desfocada cobre a tela e a nitida fica por cima (Vitrine).
 * Sem capa ou com falha de carga fica so o fundo do layout.
 */
export function Backdrop({ game, layered = false }: { game: GameDto | null; layered?: boolean }) {
  const src = game?.cover ? coverUrl(game.id) : null;
  const [failedSrc, setFailedSrc] = useState<string | null>(null);
  const show = src !== null && failedSrc !== src;
  const fail = () => setFailedSrc(src);
  return (
    <div className="backdrop" aria-hidden="true" data-testid="backdrop">
      {show && layered ? (
        <img key={`b:${src}`} className="backdrop-blur" src={src} alt="" decoding="async" draggable={false} onError={fail} />
      ) : null}
      {show ? (
        // `key`: cada jogo monta uma imagem nova, que entra com fade.
        <img key={src} className="backdrop-img" src={src} alt="" decoding="async" draggable={false} onError={fail} />
      ) : null}
      <div className="backdrop-veil" />
    </div>
  );
}
