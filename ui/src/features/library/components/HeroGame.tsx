import type { GameDto } from '@/shared/ipc';
import { useInputMode } from '@/shared/input/mode';
import { originLabel } from '@/shared/ui/Badges';
import { Icon } from '@/shared/ui/Icon';
import { lastPlayedLabel } from '../domain';

interface HeroGameProps {
  /** Jogo em foco; `null` com `allGames` = card "Todos os jogos". */
  game: GameDto | null;
  allGames?: boolean;
  now: number;
  /** Layout Vitrine: mostra o botao "Jogar" (so visual; quem joga e o A no icone em foco). */
  cta?: boolean;
}

/** Heroi fixo no alto (nome inteiro em ate 2 linhas; o card trunca, E-13). Atualiza ao mover o foco. */
export function HeroGame({ game, allGames = false, now, cta = false }: HeroGameProps) {
  const confirmKey = useInputMode((s) => (s.mode === 'keyboard' ? 'Enter' : 'A'));
  const ctaLabel = game ? (game.inProgress ? 'Voltar ao jogo' : 'Jogar') : allGames ? 'Abrir' : null;
  return (
    <div className="hero" aria-live="polite">
      <div className="eyebrow">
        {game ? (
          <>
            <span className="pill">{originLabel(game.origin)}</span>
            <span>{game.inProgress ? `Em andamento. Aperte ${confirmKey} para voltar ao jogo.` : lastPlayedLabel(game.lastPlayedAt, now)}</span>
          </>
        ) : allGames ? (
          <span>Biblioteca</span>
        ) : null}
      </div>
      <div className="name-big clamp2">{game ? game.name : allGames ? 'Todos os jogos' : ''}</div>
      {cta && ctaLabel ? (
        <div className="hero-cta" aria-hidden="true">
          <span className="hero-btn">
            <Icon name={allGames && !game ? 'grid' : 'play'} size="s" />
            <span>{ctaLabel}</span>
          </span>
          <span className="hero-key">{confirmKey}</span>
        </div>
      ) : null}
    </div>
  );
}
