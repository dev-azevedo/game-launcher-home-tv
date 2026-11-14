import { useId, type ReactNode } from 'react';
import { useBackHandler } from '@/shared/input/activation';
import { Focusable, FocusLayer } from './Focusable';
import { Icon, type IconName } from './Icon';

export type DialogTone = 'warn' | 'danger' | 'ok';

interface ConfirmDialogProps {
  title: string;
  /** Texto do corpo (pode vir com nome de jogo: renderizado como texto). */
  children?: ReactNode;
  /** Aviso extra em destaque (ex.: jogo aberto, CA-48). */
  warning?: string;
  /** `warn` (ambar) para aviso que nao e perda de dados, ex.: o jogo continua aberto ao sair. */
  warningTone?: 'danger' | 'warn';
  icon?: IconName;
  tone?: DialogTone;
  /** Ausente = dialogo de aviso com um botao so (o de cancelar, Voltar). */
  confirmLabel?: string;
  cancelLabel?: string;
  /** Obrigatorio: quem monta o dialogo decide o foco inicial (RN-14: destrutivos usam "cancel"). */
  defaultFocus: 'cancel' | 'confirm';
  /** Acao do botao de confirmar e visual destrutivo. */
  destructive?: boolean;
  onConfirm?: () => void;
  onCancel: () => void;
  active?: boolean;
}

/** Dialogo de confirmacao generico (CA-43, RN-14). B cancela. */
export function ConfirmDialog({
  title,
  children,
  warning,
  warningTone = 'danger',
  icon = 'alert',
  tone = 'warn',
  confirmLabel,
  cancelLabel = 'Cancelar',
  defaultFocus,
  destructive = false,
  onConfirm,
  onCancel,
  active = true,
}: ConfirmDialogProps) {
  const titleId = useId();
  useBackHandler(active ? onCancel : undefined);
  return (
    <FocusLayer
      focusKey="confirm-dialog"
      defaultFocus={defaultFocus === 'cancel' || !confirmLabel ? 'dlg-cancel' : 'dlg-confirm'}
      active={active}
    >
      <div className="center">
        <div className="scrim-l" />
        <div className="dialog" role="alertdialog" aria-modal="true" aria-labelledby={titleId}>
          <div className="side">
            <div className={`badge-ic${tone === 'warn' ? '' : ` ${tone}`}`}>
              <Icon name={icon} />
            </div>
          </div>
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
                  className={`btn ${destructive ? 'danger' : 'primary'}`}
                  onActivate={onConfirm}
                >
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
