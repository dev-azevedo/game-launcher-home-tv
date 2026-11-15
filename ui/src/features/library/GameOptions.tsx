import { useEffect } from 'react';
import type { ScreenEntry } from '@/app/navigation/screens';
import { useNavigation } from '@/app/navigation/store';
import { useSession } from '@/features/session/store';
import { errorCopyFor } from '@/shared/domain/errors';
import { errorOf, ipc } from '@/shared/ipc';
import { FocusLayer } from '@/shared/ui/Focusable';
import { MenuList, type MenuItemSpec } from '@/shared/ui/MenuList';
import { selectGameById, useLibrary } from './store';

interface GameOptionsProps {
  active: boolean;
  entry: ScreenEntry;
}

/**
 * Opcoes do jogo (View, CA-79, RN-33): "Jogar" primeiro e em foco, depois Renomear, Restaurar nome
 * original (so se o nome foi alterado, CA-62) e Ocultar. B fecha. Nao existe tela de detalhe.
 */
export function GameOptions({ active, entry }: GameOptionsProps) {
  const gameId = entry.params?.gameId;
  const game = useLibrary(selectGameById(gameId));
  const play = useSession((s) => s.play);
  const { push, remove } = useNavigation.getState();
  const gone = !game;

  // O jogo saiu da biblioteca com a lista aberta: fecha a camada.
  useEffect(() => {
    if (gone) remove(entry.key);
  }, [gone, remove, entry.key]);

  if (!game) return null;
  const close = () => remove(entry.key);

  const restore = async () => {
    try {
      await ipc.gameRestoreName(game.id);
      useLibrary.getState().setNotice('Nome original restaurado.');
    } catch (thrown) {
      useLibrary.getState().setNotice(errorCopyFor(errorOf(thrown)).message);
    }
    close();
  };

  const items: MenuItemSpec[] = [
    {
      id: 'play',
      label: game.inProgress ? 'Voltar ao jogo' : 'Jogar',
      icon: 'play',
      // Mesmo caminho do A no card (CA-79).
      onSelect: () => {
        close();
        void play(game);
      },
    },
    {
      id: 'rename',
      label: 'Renomear',
      icon: 'pencil',
      hint: 'Pelo teclado do PC',
      onSelect: () => push('renameDialog', undefined, { gameId: game.id }),
    },
    ...(game.originalName
      ? [
          {
            id: 'restore',
            label: 'Restaurar nome original',
            icon: 'refresh' as const,
            hint: game.originalName,
            onSelect: () => void restore(),
          },
        ]
      : []),
    {
      id: 'hide',
      label: 'Ocultar',
      icon: 'eyeoff',
      hint: 'Nenhum arquivo é apagado',
      danger: true,
      onSelect: () => push('confirmDialog', undefined, { confirm: 'hide', gameId: game.id }),
    },
  ];

  return (
    <FocusLayer focusKey="game-options" defaultFocus="menu-play" active={active}>
      <div className="center">
        <div className="scrim-l" />
        <div className="menu options">
          <div className="mh">
            <h2 className="clamp2">{game.name}</h2>
          </div>
          <MenuList label={`Opções de ${game.name}`} items={items} />
        </div>
      </div>
    </FocusLayer>
  );
}
