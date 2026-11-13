import { useState } from 'react';

/** Capa sem imagem (CA-24): gradiente da marca com o nome em Sora 700. Nome sempre como texto (SEC-07). */
export function CoverPlaceholder({ name }: { name: string }) {
  return (
    <div className="cover-ph" data-testid="cover-placeholder">
      <div className="cover-ph-name">
        <span>{name}</span>
      </div>
    </div>
  );
}

interface CoverImageProps {
  /** `cover://<gameId>` (SEC-08). Ausente = placeholder. */
  src?: string | null;
  name: string;
}

/**
 * Capa 2:3 com dimensoes fixas (sem layout shift), carga preguicosa e decodificacao assincrona.
 * Falha de carga (404, decodificacao) cai no placeholder (SEC-08, CA-24).
 */
export function CoverImage({ src, name }: CoverImageProps) {
  const [failedSrc, setFailedSrc] = useState<string | null>(null);
  if (!src || failedSrc === src) return <CoverPlaceholder name={name} />;
  return (
    <img
      className="cover-img"
      src={src}
      alt=""
      width={600}
      height={900}
      loading="lazy"
      decoding="async"
      draggable={false}
      onError={() => setFailedSrc(src)}
    />
  );
}
