import { createContext, useContext, useEffect, type ReactNode } from 'react';
import { useLatest } from '@/shared/useLatest';
import { FocusContext, useFocusable } from '@noriginmedia/norigin-spatial-navigation';
import { registerActivation } from '@/shared/input/activation';

/** So a camada do topo mostra anel de foco (handoff 7: camadas de baixo recebem data-focused="false"). */
export const LayerActiveContext = createContext(true);

interface FocusableProps {
  as?: 'div' | 'button';
  className?: string;
  focusKey?: string;
  /** Item desabilitado sai da ordem de foco (handoff 4, item 6). */
  disabled?: boolean;
  /** O que A faz neste item. */
  onActivate?: () => void;
  onFocus?: () => void;
  role?: string;
  ariaLabel?: string;
  ariaChecked?: boolean;
  children?: ReactNode;
}

/** Item focavel da marca: anel duplo + halo (tokens.css `.focusable[data-focused]`). */
export function Focusable({
  as = 'div',
  className = '',
  focusKey,
  disabled = false,
  onActivate,
  onFocus,
  role,
  ariaLabel,
  ariaChecked,
  children,
}: FocusableProps) {
  const layerActive = useContext(LayerActiveContext);
  const onFocusRef = useLatest(onFocus);
  const { ref, focused, focusKey: key } = useFocusable<object, HTMLElement>({
    focusKey,
    focusable: !disabled,
    onFocus: () => onFocusRef.current?.(),
  });

  const activateRef = useLatest(onActivate);
  useEffect(() => registerActivation(key, () => activateRef.current?.()), [key, activateRef]);

  const isFocused = focused && layerActive && !disabled;
  const common = {
    className: `focusable ${className}`.trim(),
    'data-focused': isFocused ? 'true' : 'false',
    'data-focus-key': key,
    role,
    'aria-label': ariaLabel,
    'aria-checked': ariaChecked,
    'aria-disabled': disabled || undefined,
  };

  return as === 'button' ? (
    <button ref={ref as React.RefObject<HTMLButtonElement>} type="button" tabIndex={-1} {...common}>
      {children}
    </button>
  ) : (
    <div ref={ref as React.RefObject<HTMLDivElement>} {...common}>
      {children}
    </div>
  );
}

interface FocusLayerProps {
  focusKey: string;
  /** Item que recebe o foco ao abrir (ex.: "Cancelar" em dialogos destrutivos, RN-14). */
  defaultFocus?: string;
  /** false = camada de baixo na pilha: perde o anel. */
  active?: boolean;
  /** Lembrar o ultimo filho focado ao reabrir. Padrao false: dialogos sempre abrem no item padrao (RN-14). */
  rememberFocus?: boolean;
  children: ReactNode;
}

/** Camada com arvore de foco propria (fronteira): o foco fica preso nela. */
export function FocusLayer({ focusKey, defaultFocus, active = true, rememberFocus = false, children }: FocusLayerProps) {
  const { ref, focusKey: key, focusSelf } = useFocusable<object, HTMLDivElement>({
    focusKey,
    isFocusBoundary: true,
    saveLastFocusedChild: rememberFocus,
    preferredChildFocusKey: defaultFocus,
    trackChildren: true,
  });
  useEffect(() => {
    if (active) focusSelf();
  }, [active, focusSelf]);
  return (
    <FocusContext.Provider value={key}>
      <LayerActiveContext.Provider value={active}>
        <div ref={ref} className="layer-root">
          {children}
        </div>
      </LayerActiveContext.Provider>
    </FocusContext.Provider>
  );
}
