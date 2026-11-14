import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { setFocus } from '@noriginmedia/norigin-spatial-navigation';
import { useFocusedGame } from '@/app/navigation/focusedGame';
import { usePageHandler } from '@/app/navigation/pageHandler';
import { useNavigation } from '@/app/navigation/store';
import { SafeArea } from '@/app/SafeArea';
import { useSession } from '@/features/session/store';
import type { GameDto } from '@/shared/ipc';
import { scrollBehavior } from '@/shared/theme/motion';
import { ButtonHints } from '@/shared/ui/ButtonHints';
import { FocusLayer } from '@/shared/ui/Focusable';
import { Toast } from '@/shared/ui/Toast';
import { AppHeader } from './components/AppHeader';
import { EmptyLibrary, LoadingLibrary } from './components/EmptyLibrary';
import { AllGamesCard, GameCard } from './components/GameCard';
import { HeroGame } from './components/HeroGame';
import {
  alphabetical,
  FOCUS_ALL,
  focusKeyLibrary,
  focusKeyRecent,
  initialHomeFocus,
  LIBRARY_ROW_MAX,
  recentGames,
} from './domain';
import { useLibrary } from './store';

const HOME_HINTS = [
  { glyphs: ['A' as const], label: 'Jogar' },
  { glyphs: ['view' as const], label: 'Opções' },
  { glyphs: ['menu' as const], label: 'Menu' },
  { glyphs: ['LT' as const, 'RT' as const], label: 'Volume' },
];
const EMPTY_HINTS = [
  { glyphs: ['A' as const], label: 'Escolher' },
  { glyphs: ['menu' as const], label: 'Menu' },
];

/** Inicio (CA-22, CA-23): carregando, vazio ou heroi + Recentes + Biblioteca. */
export function HomeScreen({ active, headerExtras }: { active: boolean; headerExtras?: React.ReactNode }) {
  const status = useLibrary((s) => s.status);
  const games = useLibrary((s) => s.games);
  const refreshing = useLibrary((s) => s.refreshing);
  const refresh = useLibrary((s) => s.refresh);
  const notice = useLibrary((s) => s.notice);
  const clearNotice = useLibrary((s) => s.clearNotice);
  const push = useNavigation((s) => s.push);
  const openAdd = useCallback(() => push('folderBrowser'), [push]);

  let body;
  let hints = HOME_HINTS;
  // Cache vazio com varredura em curso: esqueleto "Procurando jogos…" (handoff 5).
  if (games.length === 0 && (status === 'loading' || refreshing)) {
    body = <LoadingLibrary active={active} onAdd={openAdd} />;
    hints = EMPTY_HINTS;
  } else if (games.length === 0) {
    body = <EmptyLibrary active={active} onAdd={openAdd} onRefresh={() => void refresh()} refreshing={refreshing} />;
    hints = EMPTY_HINTS;
  } else {
    body = <HomeLibrary active={active} games={games} />;
  }

  return (
    <div className="layer home screen" data-active={active ? 'true' : 'false'} data-screen="home">
      <div className="spot" />
      <SafeArea>
        <AppHeader refreshing={refreshing}>{headerExtras}</AppHeader>
        {body}
        <ButtonHints hints={hints} />
      </SafeArea>
      <Toast message={notice} onDone={clearNotice} />
    </div>
  );
}

type HeroFocus = { kind: 'game'; id: string } | { kind: 'all' };

function HomeLibrary({ active, games }: { active: boolean; games: GameDto[] }) {
  const focusGameId = useLibrary((s) => s.focusGameId);
  const push = useNavigation((s) => s.push);
  const play = useSession((s) => s.play);
  const setFocusedGame = useFocusedGame((s) => s.set);
  const rootRef = useRef<HTMLDivElement>(null);

  const recents = useMemo(() => recentGames(games), [games]);
  const libraryRow = useMemo(() => alphabetical(games).slice(0, LIBRARY_ROW_MAX), [games]);
  const initialKey = useMemo(() => initialHomeFocus(games, focusGameId), [games, focusGameId]);

  const [hero, setHero] = useState<HeroFocus | null>(null);
  const focusedKey = useRef<string>(initialKey);
  const [now] = useState(() => Date.now());
  const heroGame = hero?.kind === 'game' ? (games.find((g) => g.id === hero.id) ?? null) : null;

  const reveal = useCallback((key: string, instant = false) => {
    const root = rootRef.current;
    if (!root) return;
    const el = Array.from(root.querySelectorAll<HTMLElement>('[data-focus-key]')).find((e) => e.dataset.focusKey === key);
    const card = el?.closest<HTMLElement>('.card');
    const row = card?.parentElement;
    if (!card || !row) return;
    const behavior = scrollBehavior(instant);
    const left = Math.max(0, Math.min(row.scrollWidth - row.clientWidth, card.offsetLeft - (row.clientWidth - card.offsetWidth) / 2));
    if (typeof row.scrollTo === 'function') row.scrollTo({ left, behavior });
    const block = row.closest<HTMLElement>('.rowblock');
    const rows = root;
    if (block && rows && typeof rows.scrollTo === 'function') {
      rows.scrollTo({ top: block.offsetTop - parseFloat(getComputedStyle(rows).paddingTop || '0'), behavior });
    }
    // Holofote acompanha a coluna do card (handoff 1).
    const screen = root.closest<HTMLElement>('.home');
    if (screen && window.innerWidth > 0) {
      const x = ((row.getBoundingClientRect().left + card.offsetLeft - left + card.offsetWidth / 2) / window.innerWidth) * 100;
      screen.style.setProperty('--spot-x', `${Math.max(12, Math.min(88, x))}%`);
    }
  }, []);

  const onFocusGame = useCallback(
    (game: GameDto, key: string) => {
      focusedKey.current = key;
      setHero({ kind: 'game', id: game.id });
      setFocusedGame(game.id);
      reveal(key);
    },
    [reveal, setFocusedGame],
  );
  const onFocusAll = useCallback(() => {
    focusedKey.current = FOCUS_ALL;
    setHero({ kind: 'all' });
    setFocusedGame(null);
    reveal(FOCUS_ALL);
  }, [reveal, setFocusedGame]);
  const openAll = useCallback(() => push('allGames'), [push]);

  // Posicao inicial das fileiras, sem animar.
  useEffect(() => reveal(initialKey, true), [reveal, initialKey]);

  // O nucleo pede foco em outro jogo (fim de sessao, CA-30) ou o jogo em foco saiu (ocultado).
  const firstRun = useRef(true);
  useEffect(() => {
    if (firstRun.current) {
      firstRun.current = false;
      return;
    }
    const current = hero?.kind === 'game' ? hero.id : null;
    const gone = current !== null && !games.some((g) => g.id === current);
    if (focusGameId || gone) setFocus(initialHomeFocus(games, focusGameId));
    // So reage a mudanca da biblioteca.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [games, focusGameId]);

  // LB/RB pulam uma linha: RB desce para a Biblioteca, LB sobe para Recentes (mesma coluna).
  usePageHandler((delta) => {
    const inRecents = focusedKey.current.startsWith('r:');
    const column = Math.max(
      0,
      (inRecents ? recents : libraryRow).findIndex((g) => focusedKey.current.endsWith(`:${g.id}`)),
    );
    if (delta > 0 && inRecents) {
      const target = libraryRow[Math.min(column, libraryRow.length - 1)];
      if (target) setFocus(focusKeyLibrary(target.id));
    } else if (delta < 0 && !inRecents && recents.length > 0) {
      const target = recents[Math.min(column, recents.length - 1)];
      if (target) setFocus(focusKeyRecent(target.id));
    }
  }, active);

  return (
    <FocusLayer focusKey="home" defaultFocus={initialKey} active={active} rememberFocus>
      <HeroGame game={heroGame} allGames={hero?.kind === 'all'} now={now} />
      <div className="rows" ref={rootRef}>
        {recents.length > 0 ? (
          <section className="rowblock" aria-label="Recentes">
            <div className="rowtitle">Recentes</div>
            <div className="row">
              {recents.map((game) => (
                <GameCard
                  key={game.id}
                  game={game}
                  focusKey={focusKeyRecent(game.id)}
                  onPlay={play}
                  onFocusGame={onFocusGame}
                />
              ))}
            </div>
          </section>
        ) : null}
        <section className="rowblock" aria-label="Biblioteca">
          <div className="rowtitle">Biblioteca</div>
          <div className="row">
            {libraryRow.map((game) => (
              <GameCard
                key={game.id}
                game={game}
                focusKey={focusKeyLibrary(game.id)}
                onPlay={play}
                onFocusGame={onFocusGame}
              />
            ))}
            <AllGamesCard count={games.length} focusKey={FOCUS_ALL} onOpen={openAll} onFocusAll={onFocusAll} />
          </div>
        </section>
      </div>
    </FocusLayer>
  );
}
