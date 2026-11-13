import { BrandSplashArt } from '@/shared/ui/Brand';

/** Primeiro paint enquanto a biblioteca carrega do cache (M-02). Passivo: sem foco. */
export function BootSplash({ status = 'Carregando biblioteca…' }: { status?: string }) {
  return (
    <div className="boot" role="status" aria-live="polite">
      <BrandSplashArt />
      <div className="boot-status">
        <div className="boot-bar" aria-hidden="true">
          <i />
        </div>
        <span>{status}</span>
      </div>
    </div>
  );
}
