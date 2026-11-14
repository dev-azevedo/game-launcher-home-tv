import { useCallback, useEffect, useMemo, useRef } from 'react';
import { setFocus } from '@noriginmedia/norigin-spatial-navigation';
import { useVirtualizer, type Virtualizer } from '@tanstack/react-virtual';
import type { GameDto } from '@/shared/ipc';
import { usePageHandler } from '@/app/navigation/pageHandler';
import { scrollBehavior } from '@/shared/theme/motion';
import { focusKeyGrid, VIRTUALIZE_ABOVE } from '../domain';
import { GameCard } from './GameCard';

export const GRID_COLUMNS = 7;
/** Altura de uma linha em unidades de 1080p: cartaz 330 + nome 34 + linha de texto 30 + folga 26. */
const ROW_UNITS = 420;
const FOCUS_PAD_UNITS = 28;
/** LB/RB pulam 3 linhas; o overscan cobre isso para o destino ja estar montado. */
const OVERSCAN_ROWS = 4;
const FALLBACK_RECT = { width: 1920, height: 700 };

const unit = () => window.innerHeight / 1080;

interface GameGridProps {
  games: GameDto[];
  active: boolean;
  initialId: string | null;
  onPlay: (game: GameDto) => void;
  onFocusGame: (game: GameDto) => void;
}

/**
 * Grade de 7 colunas, so as linhas perto da tela ficam montadas (acima de ~60 jogos, arch 3.4).
 * Rolagem so pelo foco; o anel de foco nunca e cortado (folga de 28 px nas pontas).
 */
export function GameGrid({ games, active, initialId, onPlay, onFocusGame }: GameGridProps) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const rowCount = Math.ceil(games.length / GRID_COLUMNS);
  const virtualize = games.length > VIRTUALIZE_ABOVE;
  const initialIndex = Math.max(0, initialId ? games.findIndex((g) => g.id === initialId) : 0);
  const focusedIndex = useRef(initialIndex);

  // TanStack Virtual devolve funcoes nao memoizaveis: aqui o resultado nao vai para filhos memoizados.
  // eslint-disable-next-line react-hooks/incompatible-library
  const virtualizer = useVirtualizer({
    count: rowCount,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW_UNITS * unit(),
    overscan: virtualize ? OVERSCAN_ROWS : rowCount,
    scrollPaddingStart: FOCUS_PAD_UNITS * unit(),
    scrollPaddingEnd: FOCUS_PAD_UNITS * unit(),
    initialRect: FALLBACK_RECT,
    // Foco inicial: a linha do jogo que estava em foco ja nasce montada.
    initialOffset: () => Math.floor(initialIndex / GRID_COLUMNS) * ROW_UNITS * unit(),
    // Sem layout (testes, janela oculta) o retangulo vem como 0x0: usa o inicial.
    observeElementRect: (instance, cb) => {
      const el = instance.scrollElement;
      if (!el) return;
      const read = () => {
        const r = el.getBoundingClientRect();
        cb({ width: r.width || FALLBACK_RECT.width, height: r.height || FALLBACK_RECT.height });
      };
      read();
      const ro = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(read);
      ro?.observe(el);
      return () => ro?.disconnect();
    },
  });

  // A escala (--u) muda com a janela: remede.
  const measureRef = useRef<Virtualizer<HTMLDivElement, Element>>(virtualizer);
  measureRef.current = virtualizer;
  useEffect(() => {
    const onResize = () => measureRef.current.measure();
    window.addEventListener('resize', onResize);
    return () => window.removeEventListener('resize', onResize);
  }, []);

  const indexById = useMemo(() => new Map(games.map((g, i) => [g.id, i])), [games]);

  const handleFocus = useCallback(
    (game: GameDto) => {
      const index = indexById.get(game.id) ?? 0;
      focusedIndex.current = index;
      measureRef.current.scrollToIndex(Math.floor(index / GRID_COLUMNS), {
        align: 'auto',
        behavior: scrollBehavior(),
      });
      onFocusGame(game);
    },
    [indexById, onFocusGame],
  );

  usePageHandler((delta, rows) => {
    if (games.length === 0) return;
    const target = Math.max(0, Math.min(games.length - 1, focusedIndex.current + delta * rows * GRID_COLUMNS));
    const game = games[target];
    if (game) setFocus(focusKeyGrid(game.id));
  }, active);

  return (
    <div className="grid" ref={scrollRef} role="grid" aria-label="Todos os jogos">
      <div className="grid-inner" style={{ height: virtualizer.getTotalSize() }}>
        {virtualizer.getVirtualItems().map((row) => (
          <div
            key={row.key}
            className="grid-row"
            role="row"
            style={{ height: row.size, transform: `translateY(${row.start}px)` }}
          >
            {games.slice(row.index * GRID_COLUMNS, (row.index + 1) * GRID_COLUMNS).map((game) => (
              <GameCard
                key={game.id}
                game={game}
                focusKey={focusKeyGrid(game.id)}
                onPlay={onPlay}
                onFocusGame={handleFocus}
              />
            ))}
          </div>
        ))}
      </div>
    </div>
  );
}
