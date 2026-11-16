import { useId, type ReactNode } from 'react';
import { useBackHandler } from '@/shared/input/activation';
import { Focusable, FocusLayer } from './Focusable';
import { DialogSide, type DialogCover } from './DialogSide';
import { Icon, type IconName } from './Icon';

export type DialogTone = 'warn' | 'danger' | 'ok';

interface ConfirmDialogProps {
  title: string;
  /** Nome do dialogo para leitor de tela, quando difere do titulo. Padrao: o titulo. */
  ariaLabel?: string;
  /** Texto do corpo (pode vir com nome de jogo: renderizado como texto). */
  children?: ReactNode;
  /** Aviso extra em destaque (ex.: jogo aberto, CA-48). */
  warning?: string;
  /** `warn` (ambar) para aviso que nao e perda de dados, ex.: o jogo continua aberto ao sair. */
  warningTone?: 'danger' | 'warn';
  icon?: IconName;
  tone?: DialogTone;
  /** Capa do jogo no lado esquerdo, no lugar do icone. */
  cover?: DialogCover | null;
  /** Ausente = dialogo de aviso com um botao so (o de cancelar, Voltar). */
  confirmLabel?: string;
  cancelLabel?: string;
  /** Obrigatorio: quem monta o dialogo decide o foco inicial (RN-14: destrutivos usam "cancel"). */
  defaultFocus: 'cancel' | 'confirm';
  /** Acao do botao de confirmar e visual destrutivo. */
  destructive?: boolean;
  /** Visual do botao de confirmar: `primary` (ambar), `neutral` (secundario) ou `danger`. Padrao: `destructive ? danger : primary`. */
  confirmKind?: 'primary' | 'neutral' | 'danger';
  confirmIcon?: IconName;
  onConfirm?: () => void;
  onCancel: () => void;
  active?: boolean;
}

/** Dialogo de confirmacao generico (CA-43, RN-14). B cancela. */
export function ConfirmDialog({
  title,
  ariaLabel,
  children,
  warning,
  warningTone = 'danger',
  icon = 'alert',
  tone = 'warn',
  cover,
  confirmLabel,
  cancelLabel = 'Cancelar',
  defaultFocus,
  destructive = false,
  confirmKind,
  confirmIcon,
  onConfirm,
  onCancel,
  active = true,
}: ConfirmDialogProps) {
  const titleId = useId();
  const kind = confirmKind ?? (destructive ? 'danger' : 'primary');
  useBackHandler(active ? onCancel : undefined);
  return (
    <FocusLayer
      focusKey="confirm-dialog"
      defaultFocus={defaultFocus === 'cancel' || !confirmLabel ? 'dlg-cancel' : 'dlg-confirm'}
      active={active}
    >
      <div className="center">
        <div className="scrim-l" />
        <div className="dialog" role="alertdialog" aria-modal="true" {...(ariaLabel ? { 'aria-label': ariaLabel } : { 'aria-labelledby': titleId })}>
          <DialogSide cover={cover} icon={icon} tone={tone} />
          <div className="dc">
            <h2 id={titleId}>{title}</h2>
            {children ? <div className="body">{children}</div> : null}
            {warning ? <div className={`warnline${warningTone === 'warn' ? ' amber' : ''}`}>{warning}</div> : null}
            <div className="btnrow">
              <Focusable as="button" focusKey="dlg-cancel" className="btn" onActivate={onCancel}>
                {cancelLabel}
              </Focusable>
              {confirmLabel ? (
                <Focusable
                  as="button"
                  focusKey="dlg-confirm"
                  className={kind === 'neutral' ? 'btn' : `btn ${kind}`}
                  onActivate={onConfirm}
                >
                  {confirmIcon ? <Icon name={confirmIcon} /> : null}
                  {confirmLabel}
                </Focusable>
              ) : null}
            </div>
          </div>
        </div>
      </div>
    </FocusLayer>
  );
}
