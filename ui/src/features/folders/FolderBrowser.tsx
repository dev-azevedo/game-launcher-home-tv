import { useCallback, useEffect, useRef, useState, type ReactNode } from 'react';
import { setFocus } from '@noriginmedia/norigin-spatial-navigation';
import { useVirtualizer } from '@tanstack/react-virtual';
import type { ScreenEntry } from '@/app/navigation/screens';
import { useNavigation } from '@/app/navigation/store';
import { SafeArea } from '@/app/SafeArea';
import { AppHeader } from '@/features/library/components/AppHeader';
import { useLibrary } from '@/features/library/store';
import { errorCopyFor } from '@/shared/domain/errors';
import { neutralizeName } from '@/shared/domain/text';
import { useBackHandler } from '@/shared/input/activation';
import { errorOf, ipc, type DirEntry, type DirListing, type ErrorDto } from '@/shared/ipc';
import { scrollBehavior } from '@/shared/theme/motion';
import { ButtonHints } from '@/shared/ui/ButtonHints';
import { ConfirmDialog } from '@/shared/ui/ConfirmDialog';
import { Focusable, FocusLayer } from '@/shared/ui/Focusable';
import { Icon } from '@/shared/ui/Icon';
import { entryPath, gameNameFromExe, lastSegment, parentTarget } from './domain';
import './folders.css';

const ROW_UNITS = 88; // 80 de altura + 8 de espaco
const VIRTUALIZE_ABOVE = 60;
const OVERSCAN = 8;
const FALLBACK_RECT = { width: 1400, height: 600 };
const unit = () => window.innerHeight / 1080;

type View =
  | { status: 'loading'; listing: DirListing | null }
  | { status: 'ready'; listing: DirListing; focusName?: string }
  | { status: 'error'; listing: DirListing | null; error: ErrorDto; failedName?: string };

interface Pick {
  path: string;
  fileName: string;
}

interface ListProps {
  listing: DirListing;
  focusName?: string;
  onOpen: (entry: DirEntry) => void;
}

/** Lista de entradas, so as linhas perto da tela ficam montadas acima de 60 (ate 2.000 itens, CT-04). */
function FolderList({ listing, focusName, onOpen }: ListProps) {
  const { entries } = listing;
  const scrollRef = useRef<HTMLDivElement>(null);
  const initial = Math.max(0, focusName ? entries.findIndex((e) => e.name === focusName) : 0);
  const virtualize = entries.length > VIRTUALIZE_ABOVE;

  // TanStack Virtual devolve funcoes nao memoizaveis: o resultado nao vai para filhos memoizados.
  // eslint-disable-next-line react-hooks/incompatible-library
  const virtualizer = useVirtualizer({
    count: entries.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW_UNITS * unit(),
    overscan: virtualize ? OVERSCAN : entries.length,
    initialRect: FALLBACK_RECT,
    initialOffset: () => initial * ROW_UNITS * unit(),
    observeElementRect: (instance, cb) => {
      const el = instance.scrollElement;
      if (!el) return;
      const read = () => {
        const r = el.getBoundingClientRect();
        cb({ width: r.width || FALLBACK_RECT.width, height: r.height || FALLBACK_RECT.height });
      };
      read();
      const ro = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(read);
      ro?.observe(el);
      return () => ro?.disconnect();
    },
  });

  // Foco inicial: a pasta de onde o usuario voltou (ou a primeira entrada). A linha ja nasce montada.
  useEffect(() => {
    setFocus(`fb-${initial}`);
  }, [initial]);

  return (
    <div className="fb-list" ref={scrollRef} role="list" aria-label="Pastas e arquivos">
      <div className="fb-inner" style={{ height: virtualizer.getTotalSize() }}>
        {virtualizer.getVirtualItems().map((row) => {
          const entry = entries[row.index]!;
          return (
            <div key={row.key} className="fb-row-pos" role="listitem" style={{ height: row.size, transform: `translateY(${row.start}px)` }}>
              <Focusable
                focusKey={`fb-${row.index}`}
                className="mi fb-row"
                role="button"
                onActivate={() => onOpen(entry)}
                onFocus={() =>
                  virtualizer.scrollToIndex(row.index, { align: 'auto', behavior: scrollBehavior() })
                }
              >
                <Icon name={entry.kind === 'Drive' ? 'drive' : entry.kind === 'Dir' ? 'folder' : 'exe'} />
                {/* Nome cru do arquivo: controle e bidi viram simbolo visivel (CT-03); nunca HTML. */}
                <span className="grow clamp1">{neutralizeName(entry.name)}</span>
                {entry.kind === 'Exe' ? <span className="tagsm">Adicionar</span> : <Icon name="chevron" />}
              </Focusable>
            </div>
          );
        })}
      </div>
    </div>
  );
}

/**
 * Navegador de pastas e "Adicionar jogo" (CA-19, CA-20, CA-71, CA-72, RN-29): unidades, pastas e `.exe`
 * pelo controle. So lista o que o nucleo devolve (discos deste PC); o caminho do `.exe` escolhido e a unica
 * entrada com caminho do contrato (`library_add_manual`, SEC-09). Carregado sob demanda (chunk proprio).
 */
export default function FolderBrowser({
  active,
  entry,
  headerExtras,
}: {
  active: boolean;
  entry: ScreenEntry;
  headerExtras?: ReactNode;
}) {
  const { remove } = useNavigation.getState();
  const [view, setView] = useState<View>({ status: 'loading', listing: null });
  const [pick, setPick] = useState<Pick | null>(null);
  const [addError, setAddError] = useState<ErrorDto | null>(null);
  const [adding, setAdding] = useState(false);
  const request = useRef(0);

  /** Pede a listagem; so a resposta atualiza a tela (a resposta velha de um pedido anterior e descartada). */
  const load = useCallback((path: string | null, focusName?: string) => {
    const id = ++request.current;
    ipc.fsListDir(path).then(
      (listing) => {
        if (id === request.current) setView({ status: 'ready', listing, focusName });
      },
      (thrown) => {
        if (id !== request.current) return;
        // Falha ao abrir uma pasta: a lista anterior continua valida por baixo do aviso.
        setView((v) => ({ status: 'error', listing: v.listing, error: errorOf(thrown), failedName: focusName }));
      },
    );
  }, []);
  const open = useCallback(
    (path: string | null, focusName?: string) => {
      setView((v) => ({ status: 'loading', listing: v.listing }));
      load(path, focusName);
    },
    [load],
  );

  // Primeira carga: a tela ja nasce em "loading", so falta pedir as unidades.
  useEffect(() => {
    load(null);
  }, [load]);

  const listing = view.listing;
  const close = () => remove(entry.key);

  const goBack = () => {
    if (view.status === 'loading') return;
    if (view.status === 'error') {
      // O aviso some e o foco volta a pasta que nao abriu; sem lista (falha na raiz) sai do navegador.
      if (view.listing) setView({ status: 'ready', listing: view.listing, focusName: view.failedName });
      else close();
      return;
    }
    const target = parentTarget(view.listing.path, view.listing.parent);
    if (target === undefined) close();
    else open(target, view.listing.path ? lastSegment(view.listing.path) : undefined);
  };
  useBackHandler(active && !pick ? goBack : undefined);

  const onOpen = (item: DirEntry) => {
    if (!listing) return;
    const path = entryPath(listing.path, item);
    if (item.kind === 'Exe') setPick({ path, fileName: item.name });
    else open(path);
  };

  const confirmAdd = async () => {
    if (!pick || adding) return;
    setAdding(true);
    try {
      const game = await ipc.libraryAddManual(pick.path);
      useLibrary.getState().setNotice(`${game.name} adicionado.`);
      close();
    } catch (thrown) {
      setAddError(errorOf(thrown));
    } finally {
      setAdding(false);
    }
  };

  const atRoot = listing?.path === null || listing === null;
  const hints = [
    { glyphs: ['A' as const], label: view.status === 'ready' && listing?.entries.length ? 'Abrir ou escolher' : 'Abrir' },
    { glyphs: ['B' as const], label: atRoot ? 'Voltar' : 'Pasta anterior' },
  ];

  let body: ReactNode;
  if (view.status === 'error') {
    const copy = errorCopyFor(view.error, { command: 'fs_list_dir' });
    body = (
      <FocusLayer focusKey="folder-state" defaultFocus="fb-back" active={active && !pick}>
        <div className="empty-s" role="alert">
          <Icon name="alert" size="xl" />
          <b>{copy.message}</b>
          {copy.support ? <span>{copy.support}</span> : null}
          <Focusable as="button" focusKey="fb-back" className="btn" onActivate={goBack}>
            Voltar
          </Focusable>
        </div>
      </FocusLayer>
    );
  } else if (view.status === 'ready' && view.listing.entries.length === 0) {
    body = (
      <FocusLayer focusKey="folder-state" defaultFocus="fb-back" active={active && !pick}>
        <div className="empty-s">
          <Icon name="folder" size="xl" />
          <b>Nenhuma pasta ou .exe aqui</b>
          <span>Esta pasta não tem pastas nem arquivos .exe.</span>
          <Focusable as="button" focusKey="fb-back" className="btn" onActivate={goBack}>
            Voltar
          </Focusable>
        </div>
      </FocusLayer>
    );
  } else if (view.status === 'ready') {
    body = (
      <FocusLayer focusKey="folder-browser" defaultFocus="fb-0" active={active && !pick}>
        {view.listing.truncated ? (
          <div className="callout" role="status">
            <Icon name="alert" />
            <span>A lista foi cortada. Mostrando só as primeiras {view.listing.entries.length.toLocaleString('pt-BR')} entradas desta pasta.</span>
          </div>
        ) : null}
        <FolderList
          key={view.listing.path ?? '#drives'}
          listing={view.listing}
          focusName={view.focusName}
          onOpen={onOpen}
        />
      </FocusLayer>
    );
  } else {
    body = <div className="empty-s" role="status" aria-busy="true" />;
  }

  return (
    <>
      <div className="layer home screen fb" data-active={active && !pick ? 'true' : 'false'} data-screen="folder-browser">
        <div className="spotlight" />
        <SafeArea>
          <AppHeader>{headerExtras}</AppHeader>
          <h1 className="title">Adicionar jogo</h1>
          <div className="sub">Escolha o arquivo .exe do jogo. Só aparecem discos deste PC.</div>
          <div className="crumb">
            <Icon name={atRoot ? 'drive' : 'folder'} />
            <span>{atRoot ? 'Este computador' : neutralizeName(listing.path ?? '')}</span>
          </div>
          {body}
          <ButtonHints hints={hints} />
        </SafeArea>
      </div>
      {pick && !addError ? (
        <ConfirmDialog
          key="add"
          title="Adicionar este jogo?"
          icon="plus"
          tone="ok"
          defaultFocus="confirm"
          confirmLabel="Adicionar"
          cancelLabel="Voltar"
          active={active}
          onConfirm={() => void confirmAdd()}
          onCancel={() => setPick(null)}
        >
          <p>O jogo entra na biblioteca como {neutralizeName(gameNameFromExe(pick.fileName))}. Dá para renomear depois.</p>
          <p className="path">{neutralizeName(pick.path)}</p>
        </ConfirmDialog>
      ) : null}
      {pick && addError ? (
        <ConfirmDialog
          key="add-error"
          title={errorCopyFor(addError).message}
          icon="alert"
          tone={addError.code === 'NotAnExecutable' ? 'danger' : 'warn'}
          defaultFocus="cancel"
          cancelLabel="Voltar"
          active={active}
          onCancel={() => {
            setAddError(null);
            setPick(null);
          }}
        >
          {errorCopyFor(addError).support}
        </ConfirmDialog>
      ) : null}
    </>
  );
}
