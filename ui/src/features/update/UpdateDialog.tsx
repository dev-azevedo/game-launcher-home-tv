import type { ScreenEntry } from '@/app/navigation/screens';
import { useNavigation } from '@/app/navigation/store';
import { ConfirmDialog } from '@/shared/ui/ConfirmDialog';
import { WaitScreen } from '@/shared/ui/WaitScreen';
import { progressLabel, shortNotes } from './domain';
import { useUpdate } from './store';

const UAC_WARNING = 'O Windows vai pedir permissão de administrador. Confirme com teclado ou mouse.';

/**
 * Atualizar o SofaLauncher (SEC-33): pergunta antes ("Agora não" sempre disponivel), mostra o
 * download e, no fim, o instalador fecha e reabre o launcher. Erro volta como aviso (Toast).
 */
export function UpdateDialog({ active, entry }: { active: boolean; entry: ScreenEntry }) {
  const available = useUpdate((s) => s.available);
  const phase = useUpdate((s) => s.phase);
  const progress = useUpdate((s) => s.progress);
  const { remove } = useNavigation.getState();
  const close = () => remove(entry.key);

  if (phase === 'downloading') {
    const done = progress?.total != null && progress.downloaded >= progress.total;
    return (
      <WaitScreen
        title={done ? 'Abrindo o instalador…' : 'Baixando a atualização…'}
        spinLabel={done ? 'Esperando o Windows' : 'Baixando'}
        icon="download"
        buttonLabel="Esconder"
        onLeave={close}
      >
        <p>{done ? UAC_WARNING : progressLabel(progress)}</p>
        <p>O SofaLauncher fecha e abre de novo sozinho quando terminar.</p>
      </WaitScreen>
    );
  }

  if (!available) return null;
  const notes = shortNotes(available);
  return (
    <ConfirmDialog
      title="Nova versão disponível"
      icon="download"
      tone="ok"
      defaultFocus="confirm"
      cancelLabel="Agora não"
      confirmLabel="Atualizar"
      confirmIcon="download"
      warning={UAC_WARNING}
      warningTone="warn"
      active={active}
      onConfirm={() => {
        void useUpdate
          .getState()
          .install()
          .then(close);
      }}
      onCancel={close}
    >
      <p>
        O SofaLauncher {available.version} está pronto para instalar. Você usa a versão {available.currentVersion}.
      </p>
      {notes.length ? (
        <ul className="notes">
          {notes.map((line, i) => (
            <li key={i}>{line}</li>
          ))}
        </ul>
      ) : null}
      <p>Sua biblioteca e as configurações continuam como estão.</p>
    </ConfirmDialog>
  );
}
