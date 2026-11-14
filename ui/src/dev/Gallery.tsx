import { useState } from 'react';
import { SafeArea } from '@/app/SafeArea';
import { useNavigation } from '@/app/navigation/store';
import { Banner } from '@/shared/ui/Banner';
import { InProgressBadge, OriginBadge } from '@/shared/ui/Badges';
import { ConfirmDialog } from '@/shared/ui/ConfirmDialog';
import { CoverImage } from '@/shared/ui/Cover';
import { FocusLayer } from '@/shared/ui/Focusable';
import { MenuList } from '@/shared/ui/MenuList';
import { WaitScreen } from '@/shared/ui/WaitScreen';

const NAMES = ['Minério', '<img src=x onerror=alert(1)>', 'Um título muito, mas muito longo mesmo para testar o corte do cartaz'];

/** Galeria so de desenvolvimento (#gallery): confere foco, capas, dialogo e espera no navegador. */
export default function Gallery() {
  const [overlay, setOverlay] = useState<'none' | 'confirm' | 'wait'>('none');
  const open = (kind: 'confirm' | 'wait') => {
    useNavigation.getState().push(kind === 'confirm' ? 'confirmDialog' : 'waiting');
    setOverlay(kind);
  };
  const close = () => {
    useNavigation.getState().pop();
    setOverlay('none');
  };
  return (
    <>
      <SafeArea>
        <FocusLayer focusKey="gallery" defaultFocus="menu-confirm" active={overlay === 'none'}>
          <div style={{ display: 'flex', gap: 'var(--space-4)', margin: 'var(--space-4) 0' }}>
            {NAMES.map((name, i) => (
              <div
                key={name}
                style={{
                  position: 'relative',
                  width: 'calc(220*var(--u))',
                  height: 'calc(330*var(--u))',
                  borderRadius: 'var(--radius-card)',
                  overflow: 'hidden',
                  background: 'var(--brand-surface)',
                }}
              >
                <CoverImage src={i === 0 ? 'cover://nao-existe' : null} name={name} />
                <OriginBadge origin={i === 1 ? 'Epic' : 'Steam'} />
                {i === 0 ? <InProgressBadge /> : null}
              </div>
            ))}
          </div>
          <MenuList
            label="Galeria"
            items={[
              { id: 'confirm', label: 'Abrir confirmação', icon: 'alert', danger: true, onSelect: () => open('confirm') },
              { id: 'wait', label: 'Abrir espera', icon: 'play', onSelect: () => open('wait') },
              { id: 'off', label: 'Desabilitado', icon: 'eyeoff', hint: 'Sem motivo real', disabled: true },
            ]}
          />
        </FocusLayer>
      </SafeArea>
      <Banner tone="warn">Nenhum controle conectado. Ligue o controle ou conecte o cabo.</Banner>
      {overlay === 'confirm' ? (
        <ConfirmDialog
          title="Ocultar <b>Minério</b>?"
          defaultFocus="cancel"
          confirmLabel="Ocultar"
          destructive
          tone="danger"
          icon="eyeoff"
          onConfirm={close}
          onCancel={close}
        >
          O jogo sai da biblioteca. Nenhum arquivo é apagado.
        </ConfirmDialog>
      ) : null}
      {overlay === 'wait' ? (
        <WaitScreen title="Abrindo Minério…" buttonLabel="Voltar" onLeave={close}>
          Aguarde. O jogo vai aparecer na frente.
        </WaitScreen>
      ) : null}
    </>
  );
}
