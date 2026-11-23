import { useEffect, useState } from 'react';
import { getCurrentFocusKey } from '@noriginmedia/norigin-spatial-navigation';
import { useDirectionHandler } from '@/app/navigation/directionHandler';
import type { ScreenEntry } from '@/app/navigation/screens';
import { useNavigation } from '@/app/navigation/store';
import { layoutLabel } from '@/features/appearance/domain';
import { useAppearance } from '@/features/appearance/store';
import { useLibrary } from '@/features/library/store';
import { openGameName } from '@/features/session/panel';
import { useSession } from '@/features/session/store';
import { useVolume } from '@/features/volume/store';
import { ipc, type PowerAction, type PowerCapabilities } from '@/shared/ipc';
import { BrandLogo } from '@/shared/ui/Brand';
import { ButtonHints } from '@/shared/ui/ButtonHints';
import { Slider, Switch } from '@/shared/ui/Controls';
import { FocusLayer } from '@/shared/ui/Focusable';
import { MenuList, type MenuItemSpec } from '@/shared/ui/MenuList';

const HINTS = [
  { glyphs: ['dpad' as const], label: 'Ajustar' },
  { glyphs: ['A' as const], label: 'Escolher' },
  { glyphs: ['B' as const], label: 'Fechar' },
];
const NO_AUDIO = 'Nenhum dispositivo de áudio encontrado';
const NO_POWER = 'Esta ação não está disponível neste PC.';

/**
 * Menu rapido (CA-05, CA-37 a CA-48): volume com ←/→, mudo, layout da tela inicial com ←/→, energia com confirmacao, energia, fechar o launcher e configuracoes.
 * O nivel exibido vem da store, alimentada por `volume://changed` (CA-41).
 */
export function QuickMenu({ active, entry }: { active: boolean; entry: ScreenEntry }) {
  const volume = useVolume((s) => s.volume);
  const step = useVolume((s) => s.step);
  const toggleMute = useVolume((s) => s.toggleMute);
  const session = useSession((s) => s.session);
  const games = useLibrary((s) => s.games);
  const layout = useAppearance((s) => s.layout);
  const { push, remove } = useNavigation.getState();
  const [caps, setCaps] = useState<PowerCapabilities | null>(null);

  useEffect(() => {
    void useVolume.getState().load();
    ipc.powerCapabilities().then(setCaps, () => undefined);
  }, []);

  const audioOff = volume !== null && !volume.available;
  const level = volume?.level ?? 0;
  const muted = volume?.muted ?? false;
  const inGame = openGameName(session, games) !== null;

  // ←/→ no Volume mudam 5 pontos, no Layout trocam o layout; nos demais itens a direcao segue para o foco.
  useDirectionHandler((direction) => {
    if (direction !== 'left' && direction !== 'right') return false;
    const key = getCurrentFocusKey();
    if (key === 'menu-layout') {
      useAppearance.getState().cycle(direction === 'right' ? 1 : -1);
      return true;
    }
    if (key !== 'menu-vol') return false;
    if (!audioOff) void step(direction === 'right' ? 'Up' : 'Down');
    return true;
  }, active);

  const power = (action: PowerAction, id: string, label: string, icon: MenuItemSpec['icon'], hint?: string): MenuItemSpec => {
    const off = caps !== null && caps[action.toLowerCase() as keyof PowerCapabilities] === 'Unavailable';
    return {
      id,
      label,
      icon,
      danger: true,
      disabled: off,
      hint: off ? (hint ?? NO_POWER) : undefined,
      onSelect: () => push('confirmDialog', undefined, { confirm: 'power', power: action }),
    };
  };

  const sound: MenuItemSpec[] = [
    {
      id: 'vol',
      label: 'Volume',
      icon: muted || audioOff ? 'mute' : 'vol',
      role: 'slider',
      value: { now: level, min: 0, max: 100 },
      disabled: audioOff,
      hint: audioOff ? NO_AUDIO : undefined,
      right: audioOff ? null : (
        <>
          <Slider value={level} muted={muted} />
          <span className="num">{level}</span>
        </>
      ),
    },
    {
      id: 'mute',
      label: 'Mudo',
      icon: muted ? 'mute' : 'vol',
      role: 'switch',
      checked: muted,
      disabled: audioOff,
      right: <Switch on={muted} />,
      onSelect: () => void toggleMute(),
    },
  ];
  const energy: MenuItemSpec[] = [
    power('Shutdown', 'off', 'Desligar', 'power'),
    power('Restart', 'restart', 'Reiniciar', 'refresh'),
    power('Suspend', 'suspend', 'Suspender', 'moon', 'Suspensão indisponível neste PC'),
    // Fecha so o launcher (RN-35): icone neutro, sem o vermelho dos itens que afetam o PC.
    {
      id: 'exit',
      label: 'Fechar o launcher',
      icon: 'exit',
      onSelect: () => push('confirmDialog', undefined, { confirm: 'exit' }),
    },
  ];
  const app: MenuItemSpec[] = [
    {
      id: 'layout',
      label: 'Layout',
      icon: 'layout',
      role: 'button',
      right: (
        <span className="tagsm cycler">
          <span aria-hidden="true">‹</span>
          {layoutLabel(layout)}
          <span aria-hidden="true">›</span>
        </span>
      ),
      onSelect: () => useAppearance.getState().cycle(1),
    },
    {
      id: 'settings',
      label: 'Configurações',
      icon: 'sliders',
      onSelect: () => {
        remove(entry.key);
        push('settings');
      },
    },
  ];

  return (
    <FocusLayer focusKey="quick-menu" defaultFocus={audioOff ? 'menu-off' : 'menu-vol'} active={active} rememberFocus>
      <div className="center">
        <div className="scrim-l" />
        <div className="menu options right" role="dialog" aria-label="Menu rápido">
          <div className="mh">
            <BrandLogo />
            {inGame ? <span className="pill amber push">Em andamento</span> : null}
          </div>
          <MenuList label="Som" items={sound} />
          <div className="note">
            <span className="g pill" aria-hidden="true">
              LT
            </span>
            <span className="g pill" aria-hidden="true">
              RT
            </span>
            <span>Na biblioteca, LT diminui e RT aumenta o volume.</span>
          </div>
          <div className="sep" />
          <MenuList label="Energia" items={energy} />
          <div className="sep" />
          <MenuList label="Launcher" items={app} />
          <ButtonHints hints={HINTS} />
        </div>
      </div>
    </FocusLayer>
  );
}
