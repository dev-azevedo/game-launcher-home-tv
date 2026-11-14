import { useEffect, useState } from 'react';
import type { ScreenEntry } from '@/app/navigation/screens';
import { useNavigation } from '@/app/navigation/store';
import { useBackHandler } from '@/shared/input/activation';
import { ButtonHints } from '@/shared/ui/ButtonHints';
import { CoverImage } from '@/shared/ui/Cover';
import { FocusLayer } from '@/shared/ui/Focusable';
import { MenuList, type MenuItemSpec } from '@/shared/ui/MenuList';
import { useLibrary } from '@/features/library/store';
import { playingFor } from './panel';
import { useSession } from './store';

const HINTS = [
  { glyphs: ['A' as const], label: 'Escolher' },
  { glyphs: ['B' as const], label: 'Voltar ao jogo' },
  { glyphs: ['menu' as const], label: 'Menu rápido' },
];

const MINIMIZED_NOTE =
  'O Windows não deixou o Sofá passar na frente. O jogo foi minimizado. Voltar ao jogo restaura a janela.';
const ELEVATED_NOTE = 'Este jogo roda como administrador. A combinação de retorno pode falhar nele.';

/**
 * Painel do jogo em andamento (CA-32, CA-34 a CA-37): "Voltar ao jogo" em foco (B faz o mesmo),
 * "Fechar jogo" com confirmacao, "Ir para a biblioteca" (o jogo segue aberto, sem minimizar) e menu rapido.
 */
export function SessionPanel({ active, entry }: { active: boolean; entry: ScreenEntry }) {
  const session = useSession((s) => s.session);
  const outcome = useSession((s) => s.panelOutcome);
  const playing = useSession((s) => s.playing);
  const returnToGame = useSession((s) => s.returnToGame);
  const game = useLibrary((s) => s.games.find((g) => g.id === session.gameId));
  const { push, remove, reset } = useNavigation.getState();
  const [now, setNow] = useState(() => Date.now());
  const [busy, setBusy] = useState(false);
  const idle = session.state === 'Idle' || session.state === 'Ending';

  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 30_000);
    return () => clearInterval(timer);
  }, []);

  // O jogo fechou por conta propria com o painel aberto: nada mais a mostrar.
  useEffect(() => {
    if (idle) remove(entry.key);
  }, [idle, remove, entry.key]);

  const back = () => {
    if (busy) return;
    setBusy(true);
    // Sucesso fecha o painel dentro do store; erro abre o dialogo e o painel segue por baixo.
    void returnToGame().finally(() => setBusy(false));
  };
  useBackHandler(active ? back : undefined);

  if (idle) return null;
  const name = game?.name ?? 'O jogo';
  const note = session.minimized || outcome === 'FrontAfterMinimize' ? MINIMIZED_NOTE : session.elevated ? ELEVATED_NOTE : null;

  const items: MenuItemSpec[] = [
    { id: 'back', label: 'Voltar ao jogo', icon: 'play', onSelect: back },
    {
      id: 'close',
      label: 'Fechar jogo',
      icon: 'power',
      danger: true,
      onSelect: () => push('confirmDialog', undefined, { confirm: 'close', gameId: session.gameId ?? undefined }),
    },
    { id: 'library', label: 'Ir para a biblioteca', icon: 'grid', hint: 'O jogo continua aberto', onSelect: reset },
    { id: 'quick', label: 'Menu rápido', icon: 'sliders', onSelect: () => push('quickMenu') },
  ];

  return (
    <FocusLayer focusKey="session-panel" defaultFocus="menu-back" active={active} rememberFocus>
      <div className="center">
        <div className="scrim-l" />
        <div className="menu options" role="dialog" aria-label="Jogo em andamento">
          <div className="panel-hero">
            <div className="thumb">
              <CoverImage gameId={game?.cover ? game.id : null} name={name} />
            </div>
            <div className="ph1">
              <div className="lab">Em andamento</div>
              <h2 className="clamp2">{name}</h2>
              <div className="sub" aria-live="polite">
                {playingFor(playing?.gameId === session.gameId ? playing.since : session.since, now)}
              </div>
            </div>
          </div>
          <MenuList label="Jogo em andamento" items={items} />
          {note ? (
            <div className="minnote" role="status">
              {note}
            </div>
          ) : null}
          <ButtonHints hints={HINTS} />
        </div>
      </div>
    </FocusLayer>
  );
}
