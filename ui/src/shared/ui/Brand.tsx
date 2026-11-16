import logoHorizontal from '@brand/assets/logo-horizontal.svg';
import symbol from '@brand/assets/symbol.svg';
import wordmark from '@brand/assets/wordmark.svg';
import splash from '@brand/assets/splash.svg';

/** Assets locais da marca como URL (img-src 'self'). Sem copia: alias @brand. */
export function BrandLogo({ className = 'brand-logo' }: { className?: string }) {
  return <img className={className} src={logoHorizontal} alt="SofaLauncher" draggable={false} />;
}

export function BrandSymbol({ className = 'brand-symbol' }: { className?: string }) {
  return <img className={className} src={symbol} alt="SofaLauncher" draggable={false} />;
}

export function BrandWordmark({ className = 'brand-wordmark' }: { className?: string }) {
  return <img className={className} src={wordmark} alt="SofaLauncher" draggable={false} />;
}

export function BrandSplashArt() {
  return <img className="splash-art" src={splash} alt="" draggable={false} />;
}
