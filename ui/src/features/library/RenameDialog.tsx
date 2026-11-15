import { useId, useRef, useState } from 'react';
import type { ScreenEntry } from '@/app/navigation/screens';
import { useNavigation } from '@/app/navigation/store';
import { errorCopyFor } from '@/shared/domain/errors';
import { useBackHandler } from '@/shared/input/activation';
import { errorOf, ipc } from '@/shared/ipc';
import { Focusable, FocusLayer } from '@/shared/ui/Focusable';
import { selectGameById, useLibrary } from './store';

interface Props {
  active: boolean;
  entry: ScreenEntry;
  parentKey?: string;
}

const MAX_NAME = 256;
const EMPTY = 'O nome não pode ficar vazio.';

/**
 * Renomear (CA-61, carregada sob demanda): o campo usa o teclado fisico. Enter salva, Esc e B
 * cancelam, nome vazio e recusado. Dentro do campo so Enter, Esc e as setas verticais saem dele.
 */
export default function RenameDialog({ active, entry, parentKey }: Props) {
  const gameId = entry.params?.gameId;
  const game = useLibrary(selectGameById(gameId));
  const titleId = useId();
  const inputRef = useRef<HTMLInputElement>(null);
  const [value, setValue] = useState(game?.name ?? '');
  const [error, setError] = useState<string | null>(null);
  const { remove } = useNavigation.getState();

  const cancel = () => remove(entry.key);
  useBackHandler(active ? cancel : undefined);

  const save = async () => {
    if (!game) return;
    const name = value.trim();
    if (!name) {
      setError(EMPTY);
      inputRef.current?.focus();
      return;
    }
    try {
      await ipc.gameRename(game.id, name);
      useLibrary.getState().setNotice('Nome salvo.');
      remove(entry.key);
      if (parentKey) remove(parentKey);
    } catch (thrown) {
      const e = errorOf(thrown);
      setError(e.code === 'EmptyName' ? EMPTY : errorCopyFor(e).message);
    }
  };

  const leaveField = () => inputRef.current?.blur();

  return (
    <FocusLayer focusKey="rename-dialog" defaultFocus="rename-field" active={active}>
      <div className="center">
        <div className="scrim-l" />
        <div className="dialog" role="dialog" aria-modal="true" aria-labelledby={titleId}>
          <div className="dc">
            <h2 id={titleId}>Renomear jogo</h2>
            {game?.originalName ? <div className="body">Nome original: {game.originalName}</div> : null}
            <Focusable
              focusKey="rename-field"
              className="field"
              onFocus={() => inputRef.current?.focus()}
              onActivate={() => void save()}
            >
              <input
                ref={inputRef}
                type="text"
                value={value}
                maxLength={MAX_NAME}
                aria-label="Novo nome"
                aria-invalid={error ? true : undefined}
                autoComplete="off"
                spellCheck={false}
                onChange={(e) => {
                  setValue(e.target.value);
                  setError(null);
                }}
              />
            </Focusable>
            {error ? (
              <div className="warnline" role="alert">
                {error}
              </div>
            ) : (
              <div className="body">Digite no teclado do PC. Enter salva, Esc cancela.</div>
            )}
            <div className="btnrow">
              <Focusable as="button" focusKey="rename-cancel" className="btn" onFocus={leaveField} onActivate={cancel}>
                Cancelar
              </Focusable>
              <Focusable
                as="button"
                focusKey="rename-save"
                className="btn primary"
                onFocus={leaveField}
                onActivate={() => void save()}
              >
                Salvar
              </Focusable>
            </div>
          </div>
        </div>
      </div>
    </FocusLayer>
  );
}
