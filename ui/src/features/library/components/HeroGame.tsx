import type { GameDto } from '@/shared/ipc';
import { originLabel } from '@/shared/ui/Badges';
import { lastPlayedLabel } from '../domain';

interface HeroGameProps {
  /** Jogo em foco; `null` com `allGames` = card "Todos os jogos". */
  game: GameDto | null;
  allGames?: boolean;
  now: number;
}

/** Heroi fixo no alto (nome inteiro em ate 2 linhas; o card trunca, E-13). Atualiza ao mover o foco. */
export function HeroGame({ game, allGames = false, now }: HeroGameProps) {
  return (
    <div className="hero" aria-live="polite">
      <div className="eyebrow">
        {game ? (
          <>
            <span className="pill">{originLabel(game.origin)}</span>
            <span>{game.inProgress ? 'Em andamento. A volta ao jogo.' : lastPlayedLabel(game.lastPlayedAt, now)}</span>
          </>
        ) : allGames ? (
          <span>Biblioteca</span>
        ) : null}
      </div>
      <div className="name-big clamp2">{game ? game.name : allGames ? 'Todos os jogos' : ''}</div>
    </div>
  );
}
