import { useCallback, useMemo, useState } from 'react';
import { useFocusedGame } from '@/app/navigation/focusedGame';
import { SafeArea } from '@/app/SafeArea';
import { useSession } from '@/features/session/store';
import type { GameDto } from '@/shared/ipc';
import { ButtonHints } from '@/shared/ui/ButtonHints';
import { FocusLayer } from '@/shared/ui/Focusable';
import { AppHeader } from './components/AppHeader';
import { GameGrid } from './components/GameGrid';
import { alphabetical, focusKeyGrid } from './domain';
import { useLibrary } from './store';

const HINTS = [
  { glyphs: ['A' as const], label: 'Jogar' },
  { glyphs: ['view' as const], label: 'Opções' },
  { glyphs: ['LB' as const, 'RB' as const], label: 'Pular linhas' },
  { glyphs: ['B' as const], label: 'Voltar' },
];

/** Todos os jogos (CA-22): grade de 7 colunas de A a Z, virtualizada acima de ~60 jogos. */
export function AllGamesScreen({ active, headerExtras }: { active: boolean; headerExtras?: React.ReactNode }) {
  const games = useLibrary((s) => s.games);
  const refreshing = useLibrary((s) => s.refreshing);
  const play = useSession((s) => s.play);
  const setFocusedGame = useFocusedGame((s) => s.set);
  // Foco de partida: o jogo que estava em foco na Home (lido uma vez, ao abrir).
  const [initialId] = useState(() => useFocusedGame.getState().gameId);

  const sorted = useMemo(() => alphabetical(games), [games]);
  const onFocusGame = useCallback((game: GameDto) => setFocusedGame(game.id), [setFocusedGame]);
  const first = sorted[0];
  const start = sorted.some((g) => g.id === initialId) ? initialId : (first?.id ?? null);

  return (
    <div className="layer home screen" data-active={active ? 'true' : 'false'} data-screen="all-games">
      <div className="spotlight" />
      <SafeArea>
        <AppHeader refreshing={refreshing}>{headerExtras}</AppHeader>
        <div className="grid-head">
          <h1 className="title">Todos os jogos</h1>
          <span className="sub">{sorted.length} jogos, de A a Z</span>
        </div>
        <FocusLayer
          focusKey="all-games"
          defaultFocus={start ? focusKeyGrid(start) : undefined}
          active={active}
          rememberFocus
        >
          <GameGrid games={sorted} active={active} initialId={start} onPlay={play} onFocusGame={onFocusGame} />
        </FocusLayer>
        <ButtonHints hints={HINTS} />
      </SafeArea>
    </div>
  );
}
