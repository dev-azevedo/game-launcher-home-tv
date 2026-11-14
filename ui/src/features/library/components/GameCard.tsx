import { memo } from 'react';
import type { GameDto } from '@/shared/ipc';
import { InProgressBadge, originLabel, OriginBadge } from '@/shared/ui/Badges';
import { CoverImage } from '@/shared/ui/Cover';
import { Focusable } from '@/shared/ui/Focusable';
import { Icon } from '@/shared/ui/Icon';

interface GameCardProps {
  game: GameDto;
  focusKey: string;
  onPlay: (game: GameDto) => void;
  onFocusGame: (game: GameDto, focusKey: string) => void;
  /** Card em foco: destaca o nome (o resto fica a 72%). */
  on?: boolean;
}

/**
 * Cartaz 2:3 (CA-60): capa ou placeholder, origem embaixo, "Em andamento" no alto (CA-36).
 * Nome em 1 linha com reticencias (E-13), sempre como texto (SEC-07).
 */
export const GameCard = memo(function GameCard({ game, focusKey, onPlay, onFocusGame, on }: GameCardProps) {
  return (
    <div className="card" data-on={on ? 'true' : 'false'} data-game-card>
      <Focusable
        focusKey={focusKey}
        className="poster"
        role="button"
        ariaLabel={`${game.name}, ${originLabel(game.origin)}${game.inProgress ? ', em andamento' : ''}`}
        onActivate={() => onPlay(game)}
        onFocus={() => onFocusGame(game, focusKey)}
      >
        <CoverImage gameId={game.cover ? game.id : null} name={game.name} />
        <div className="scrim" />
        <OriginBadge origin={game.origin} />
        {game.inProgress ? <InProgressBadge /> : null}
      </Focusable>
      <div className="nm clamp1">{game.name}</div>
    </div>
  );
});

interface AllGamesCardProps {
  count: number;
  onOpen: () => void;
  onFocusAll: () => void;
  focusKey: string;
}

/** Ultimo card da fileira Biblioteca: abre "Todos os jogos". */
export const AllGamesCard = memo(function AllGamesCard({ count, onOpen, onFocusAll, focusKey }: AllGamesCardProps) {
  return (
    <div className="card" data-game-card>
      <Focusable
        focusKey={focusKey}
        className="poster all"
        role="button"
        ariaLabel={`Todos os jogos, ${count}`}
        onActivate={onOpen}
        onFocus={onFocusAll}
      >
        <Icon name="grid" size="l" />
        <span>Todos os jogos</span>
        <span className="muted">{count}</span>
      </Focusable>
      <div className="nm">{' '}</div>
    </div>
  );
});
