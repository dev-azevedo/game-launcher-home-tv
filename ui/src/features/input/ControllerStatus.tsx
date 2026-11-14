import { Banner } from '@/shared/ui/Banner';
import { Icon } from '@/shared/ui/Icon';
import { useControllers } from './store';

const TEXT = {
  warn: 'Nenhum controle conectado. Ligue o controle ou conecte o cabo.',
  lost: 'Controle desconectado. Ligue o controle ou conecte o cabo.',
  ok: 'Controle conectado.',
} as const;

/** Faixa no topo sem tomar foco (CA-13 a CA-15): o foco atual segue onde estava. */
export function ControllerBanner() {
  const banner = useControllers((s) => s.banner);
  if (banner === 'none') return null;
  return <Banner tone={banner}>{TEXT[banner]}</Banner>;
}

/** Indicador no cabecalho: "Controle conectado" / "Sem controle". */
export function ControllerChip() {
  const connected = useControllers((s) => s.connected);
  if (connected === null) return null;
  const bad = connected === 0;
  return (
    <span className={`chip${bad ? ' bad' : ''}`}>
      <Icon name="gamepad" size="s" />
      <span>{bad ? 'Sem controle' : 'Controle conectado'}</span>
    </span>
  );
}
