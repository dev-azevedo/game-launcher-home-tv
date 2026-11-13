import { useEffect, useId, useState, type ReactNode } from 'react';
import { useBackHandler } from '@/shared/input/activation';
import { timing } from '@/shared/theme/tokens';
import { Focusable, FocusLayer } from './Focusable';

interface WaitScreenProps {
  /** Ex.: "Abrindo <nome>…" (texto, SEC-07). */
  title: string;
  children?: ReactNode;
  /** Unico botao focavel ("Voltar", "Ir para a biblioteca"). */
  buttonLabel: string;
  /** So interrompe a espera; nao cancela o inicio nem o fechamento do jogo (RN-31). */
  onLeave: () => void;
  guardMs?: number;
}

/**
 * Base das telas de espera (RN-31, CA-77, CA-78): um botao em foco; A e ignorado nos
 * primeiros 600 ms (toque atrasado), B funciona desde o inicio.
 */
export function WaitScreen({
  title,
  children,
  buttonLabel,
  onLeave,
  guardMs = timing.waitScreenGuardMs,
}: WaitScreenProps) {
  const titleId = useId();
  const [armed, setArmed] = useState(guardMs <= 0);
  useEffect(() => {
    if (guardMs <= 0) return;
    const timer = setTimeout(() => setArmed(true), guardMs);
    return () => clearTimeout(timer);
  }, [guardMs]);

  useBackHandler(onLeave);

  return (
    <FocusLayer focusKey="wait-screen" defaultFocus="wait-leave">
      <div className="center">
        <div className="scrim-l" />
        <div className="dialog" role="alertdialog" aria-modal="true" aria-labelledby={titleId}>
          <div className="dc">
            <h2 id={titleId}>{title}</h2>
            <div className="rowspin">
              <div className="spinner" aria-hidden="true" />
              <div className="body">{children}</div>
            </div>
            <div className="btnrow">
              <Focusable
                as="button"
                focusKey="wait-leave"
                className="btn"
                onActivate={() => {
                  if (armed) onLeave();
                }}
              >
                {buttonLabel}
              </Focusable>
            </div>
          </div>
        </div>
      </div>
    </FocusLayer>
  );
}
