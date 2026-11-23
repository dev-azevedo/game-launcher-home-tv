import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { setFocus } from '@noriginmedia/norigin-spatial-navigation';
import { useFocusedGame } from '@/app/navigation/focusedGame';
import { usePageHandler } from '@/app/navigation/pageHandler';
import { useNavigation } from '@/app/navigation/store';
import { SafeArea } from '@/app/SafeArea';
import { useAppearance } from '@/features/appearance/store';
import type { LayoutId } from '@/features/appearance/domain';
import { useSession } from '@/features/session/store';
import type { GameDto } from '@/shared/ipc';
import { scrollBehavior } from '@/shared/theme/motion';
import { ButtonHints } from '@/shared/ui/ButtonHints';
import { Focusable, FocusLayer } from '@/shared/ui/Focusable';
import { Icon } from '@/shared/ui/Icon';
import { AppHeader } from './components/AppHeader';
import { ConsoleHeader } from './components/ConsoleHeader';
import { Backdrop } from './components/Backdrop';
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
  stripGames,
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

/** Botoes do heroi do layout Vitrine. */
export const FOCUS_HERO_PLAY = 'hero-play';
export const FOCUS_HERO_MORE = 'hero-more';

/** Inicio (CA-22, CA-23): carregando, vazio ou heroi + Recentes + Biblioteca. */
export function HomeScreen({ active, headerExtras }: { active: boolean; headerExtras?: React.ReactNode }) {
  const status = useLibrary((s) => s.status);
  const games = useLibrary((s) => s.games);
  const refreshing = useLibrary((s) => s.refreshing);
  const refresh = useLibrary((s) => s.refresh);
  const layout = useAppearance((s) => s.layout);
  const push = useNavigation((s) => s.push);
  const openAdd = useCallback(() => push('folderBrowser'), [push]);

  let body;
  let hints = HOME_HINTS;
  // Layouts de console trazem a propria barra do topo, dentro da camada de foco (o botao de configuracoes e focavel).
  let consoleHeader = false;
  // Cache vazio com varredura em curso: esqueleto "Procurando jogos…" (handoff 5).
  if (games.length === 0 && (status === 'loading' || refreshing)) {
    body = <LoadingLibrary active={active} onAdd={openAdd} />;
    hints = EMPTY_HINTS;
  } else if (games.length === 0) {
    body = <EmptyLibrary active={active} onAdd={openAdd} onRefresh={() => void refresh()} refreshing={refreshing} />;
    hints = EMPTY_HINTS;
  } else {
    // `key`: trocar de layout remonta a fileira (foco e rolagem recomecam do jogo atual).
    body = (
      <HomeLibrary key={layout} active={active} games={games} layout={layout} refreshing={refreshing} headerExtras={headerExtras} />
    );
    consoleHeader = layout !== 'sofa';
  }

  return (
    <div className="layer home screen" data-active={active ? 'true' : 'false'} data-screen="home" data-layout={layout}>
      <div className="spot" />
      <SafeArea>
        {consoleHeader ? null : <AppHeader refreshing={refreshing}>{headerExtras}</AppHeader>}
        {body}
        <ButtonHints hints={hints} />
      </SafeArea>
    </div>
  );
}

type HeroFocus = { kind: 'game'; id: string } | { kind: 'all' };

/**
 * Corpo da Home com jogos. A estrutura muda com o layout:
 * - `sofa`: heroi no alto + Recentes + Biblioteca (cartazes 2:3);
 * - `vitrine` (disposicao do PS5): barra com aba "Jogos", relogio e configuracoes; uma fileira de icones (Recentes e
 *   depois A-Z) com o nome ao lado do icone em foco; arte ao fundo; nome grande com "Jogar" e "⋯" embaixo;
 * - `mosaico` (disposicao do Xbox Series): barra com avatar, configuracoes e relogio; arte ao fundo na parte de cima;
 *   primeira fileira sem titulo com o ultimo jogado em bloco grande e "Meus jogos" no fim; depois a Biblioteca.
 * As chaves de foco sao as mesmas nos tres (r:, a:, all), entao foco inicial e retorno de sessao nao mudam.
 */
function HomeLibrary({
  active,
  games,
  layout,
  refreshing,
  headerExtras,
}: {
  active: boolean;
  games: GameDto[];
  layout: LayoutId;
  refreshing: boolean;
  headerExtras?: React.ReactNode;
}) {
  const focusGameId = useLibrary((s) => s.focusGameId);
  const push = useNavigation((s) => s.push);
  const play = useSession((s) => s.play);
  const setFocusedGame = useFocusedGame((s) => s.set);
  const rootRef = useRef<HTMLDivElement>(null);

  const recents = useMemo(() => recentGames(games), [games]);
  const libraryRow = useMemo(() => alphabetical(games).slice(0, LIBRARY_ROW_MAX), [games]);
  // Vitrine: a fileira A-Z sem repetir quem ja esta em Recentes.
  const strip = useMemo(() => stripGames(recents, libraryRow), [recents, libraryRow]);
  const single = layout === 'vitrine';
  const initialKey = useMemo(() => initialHomeFocus(games, focusGameId), [games, focusGameId]);

  const [heroFocus, setHero] = useState<HeroFocus | null>(null);
  const focusedKey = useRef<string>(initialKey);
  const [now] = useState(() => Date.now());
  const heroGame = heroFocus?.kind === 'game' ? (games.find((g) => g.id === heroFocus.id) ?? null) : null;

  const reveal = useCallback((key: string, instant = false) => {
    const root = rootRef.current;
    if (!root) return;
    const el = Array.from(root.querySelectorAll<HTMLElement>('[data-focus-key]')).find((e) => e.dataset.focusKey === key);
    const card = el?.closest<HTMLElement>('.card');
    const row = card?.parentElement;
    if (!card || !row) return;
    const behavior = scrollBehavior(instant);
    let left: number;
    if (single) {
      // Vitrine: o icone em foco fica na ponta esquerda. A conta usa o indice (todos tem a mesma largura
      // base), porque a margem do icone que perdeu o foco ainda esta em transicao.
      const index = Array.prototype.indexOf.call(row.children, card);
      const gap = parseFloat(getComputedStyle(row).columnGap || '0') || 0;
      left = Math.max(0, index * (card.offsetWidth + gap));
    } else {
      left = Math.max(0, Math.min(row.scrollWidth - row.clientWidth, card.offsetLeft - (row.clientWidth - card.offsetWidth) / 2));
    }
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
  }, [single]);

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
    const current = heroFocus?.kind === 'game' ? heroFocus.id : null;
    const gone = current !== null && !games.some((g) => g.id === current);
    if (focusGameId || gone) setFocus(initialHomeFocus(games, focusGameId));
    // So reage a mudanca da biblioteca.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [games, focusGameId]);

  // LB/RB pulam uma linha: RB desce para a Biblioteca, LB sobe para Recentes (mesma coluna).
  usePageHandler((delta) => {
    if (single) {
      // Uma fileira so: RB vai ao primeiro jogo de A-Z, LB volta ao primeiro recente.
      const target = delta > 0 ? strip.find((g) => !recents.includes(g)) : recents[0];
      if (target) setFocus(delta > 0 ? focusKeyLibrary(target.id) : focusKeyRecent(target.id));
      return;
    }
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

  const allGames = heroFocus?.kind === 'all';
  const allCard = (
    <AllGamesCard
      count={games.length}
      focusKey={FOCUS_ALL}
      onOpen={openAll}
      onFocusAll={onFocusAll}
      label={layout === 'sofa' ? undefined : layout === 'vitrine' ? 'Biblioteca de jogos' : 'Meus jogos'}
    />
  );
  // Vitrine: o jogo selecionado continua destacado quando o foco desce para "Jogar" ou sobe para a barra.
  const selectedId = single ? (heroGame?.id ?? null) : null;
  const card = (game: GameDto, key: string, featured = false) => (
    <GameCard
      key={key}
      game={game}
      focusKey={key}
      onPlay={play}
      onFocusGame={onFocusGame}
      featured={featured}
      on={selectedId === game.id}
    />
  );

  if (layout === 'sofa') {
    return (
      <FocusLayer focusKey="home" defaultFocus={initialKey} active={active} rememberFocus>
        <HeroGame game={heroGame} allGames={allGames} now={now} />
        <div className="rows" ref={rootRef}>
          {recents.length > 0 ? (
            <section className="rowblock" aria-label="Recentes">
              <div className="rowtitle">Recentes</div>
              <div className="row">{recents.map((game) => card(game, focusKeyRecent(game.id)))}</div>
            </section>
          ) : null}
          <section className="rowblock" aria-label="Biblioteca">
            <div className="rowtitle">Biblioteca</div>
            <div className="row">
              {libraryRow.map((game) => card(game, focusKeyLibrary(game.id)))}
              {allCard}
            </div>
          </section>
        </div>
      </FocusLayer>
    );
  }

  const header = (
    <ConsoleHeader layout={layout} refreshing={refreshing}>
      {headerExtras}
    </ConsoleHeader>
  );

  if (single) {
    // Vitrine: botoes "Jogar" e "⋯" abaixo do nome, como no console (↓ a partir do icone).
    const actions = heroGame ? (
      <>
        <Focusable focusKey={FOCUS_HERO_PLAY} className="hero-btn" role="button" onActivate={() => play(heroGame)}>
          <Icon name="play" size="s" />
          <span>{heroGame.inProgress ? 'Voltar ao jogo' : 'Jogar'}</span>
        </Focusable>
        <Focusable
          focusKey={FOCUS_HERO_MORE}
          className="hero-more"
          role="button"
          ariaLabel="Opções do jogo"
          onActivate={() => push('gameOptions', FOCUS_HERO_MORE, { gameId: heroGame.id })}
        >
          <Icon name="more" size="s" />
        </Focusable>
      </>
    ) : allGames ? (
      <Focusable focusKey={FOCUS_HERO_PLAY} className="hero-btn" role="button" onActivate={openAll}>
        <Icon name="grid" size="s" />
        <span>Abrir</span>
      </Focusable>
    ) : null;
    const recentIds = new Set(recents.map((g) => g.id));
    return (
      <FocusLayer focusKey="home" defaultFocus={initialKey} active={active} rememberFocus>
        <Backdrop game={heroGame} layered />
        {header}
        <div className="rows" ref={rootRef}>
          <section className="rowblock" aria-label="Jogos">
            <div className="row">
              {strip.map((game) => card(game, recentIds.has(game.id) ? focusKeyRecent(game.id) : focusKeyLibrary(game.id)))}
              {allCard}
            </div>
          </section>
        </div>
        <HeroGame game={heroGame} allGames={allGames} now={now} actions={actions} />
      </FocusLayer>
    );
  }

  // Mosaico: primeira fileira sem titulo (ultimo jogado em bloco grande + "Meus jogos" no fim), depois Biblioteca.
  const hasRecents = recents.length > 0;
  return (
    <FocusLayer focusKey="home" defaultFocus={initialKey} active={active} rememberFocus>
      <Backdrop game={heroGame} />
      {header}
      <div className="c-stage" aria-live="polite">
        <span className="clamp1">{heroGame ? heroGame.name : allGames ? 'Todos os jogos' : ''}</span>
      </div>
      <div className="rows" ref={rootRef}>
        {hasRecents ? (
          <section className="rowblock first" aria-label="Recentes">
            <div className="row">
              {recents.map((game, i) => card(game, focusKeyRecent(game.id), i === 0))}
              {allCard}
            </div>
          </section>
        ) : null}
        <section className="rowblock" aria-label="Biblioteca">
          <div className="rowtitle">Biblioteca</div>
          <div className="row">
            {libraryRow.map((game) => card(game, focusKeyLibrary(game.id)))}
            {hasRecents ? null : allCard}
          </div>
        </section>
      </div>
    </FocusLayer>
  );
}
