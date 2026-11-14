import type { ReactNode } from 'react';
import { Focusable } from './Focusable';
import { Icon, type IconName } from './Icon';

export interface MenuItemSpec {
  id: string;
  label: string;
  icon?: IconName;
  /** Linha de apoio (ex.: "Nenhum arquivo e apagado"). */
  hint?: string;
  /** Controle a direita (Switch, Slider, chevron). So visual: o foco e da linha. */
  right?: ReactNode;
  danger?: boolean;
  /** Desabilitado: sai da ordem de foco e mostra o motivo em `hint`. */
  disabled?: boolean;
  /** Padrao `menuitem`; `slider` (volume) e `switch` (mudo, ligar com o Windows) quando o item e um controle. */
  role?: 'menuitem' | 'slider' | 'switch' | 'button';
  checked?: boolean;
  value?: { now: number; min: number; max: number };
  onSelect?: () => void;
  /** O foco chegou na linha (ex.: configuracoes sabem em que zona estao). */
  onFocus?: () => void;
}

interface MenuListProps {
  items: MenuItemSpec[];
  label: string;
}

/** Lista vertical de acoes (role="menu"). Rotulos e dicas sao texto (SEC-07). */
export function MenuList({ items, label }: MenuListProps) {
  return (
    <div role="menu" aria-label={label} className="menu-list">
      {items.map((item) => (
        <Focusable
          key={item.id}
          focusKey={`menu-${item.id}`}
          className={`mi${item.danger ? ' danger' : ''}`}
          role={item.role ?? 'menuitem'}
          ariaChecked={item.role === 'switch' ? (item.checked ?? false) : undefined}
          ariaValue={item.value}
          ariaLabel={item.role === 'slider' ? item.label : undefined}
          disabled={item.disabled}
          onActivate={item.onSelect}
          onFocus={item.onFocus}
        >
          {item.icon ? <Icon name={item.icon} /> : null}
          <span className="grow">
            <span className="clamp1 mi-label">{item.label}</span>
            {item.hint ? <span className="small">{item.hint}</span> : null}
          </span>
          {item.right}
        </Focusable>
      ))}
    </div>
  );
}
