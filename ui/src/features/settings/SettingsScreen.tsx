import { useEffect, useMemo, useState, type ReactNode } from 'react';
import { getCurrentFocusKey, setFocus } from '@noriginmedia/norigin-spatial-navigation';
import { useDirectionHandler } from '@/app/navigation/directionHandler';
import type { ScreenEntry } from '@/app/navigation/screens';
import { useNavigation } from '@/app/navigation/store';
import { SafeArea } from '@/app/SafeArea';
import { LAYOUTS } from '@/features/appearance/domain';
import { useAppearance } from '@/features/appearance/store';
import { AppHeader } from '@/features/library/components/AppHeader';
import { useLibrary } from '@/features/library/store';
import { useUpdate } from '@/features/update/store';
import { useBackHandler } from '@/shared/input/activation';
import { events } from '@/shared/ipc';
import { scrollBehavior } from '@/shared/theme/motion';
import { originLabel } from '@/shared/ui/Badges';
import { ButtonHints } from '@/shared/ui/ButtonHints';
import { Switch } from '@/shared/ui/Controls';
import { Focusable, FocusLayer } from '@/shared/ui/Focusable';
import { Icon, type IconName } from '@/shared/ui/Icon';
import { MenuList, type MenuItemSpec } from '@/shared/ui/MenuList';
import { APP_FOOTER, APP_VERSION, chosenMonitorMissing, librarySummary, mechanismLabel } from './domain';
import { useSettings } from './store';
import './settings.css';

type TabId = 'screen' | 'appearance' | 'library' | 'general' | 'hidden';
const TABS: ReadonlyArray<{ id: TabId; label: string; icon: IconName }> = [
  { id: 'screen', label: 'Tela', icon: 'monitor' },
  { id: 'appearance', label: 'Aparência', icon: 'layout' },
  { id: 'library', label: 'Biblioteca', icon: 'grid' },
  { id: 'general', label: 'Geral', icon: 'sliders' },
  { id: 'hidden', label: 'Itens ocultos', icon: 'eyeoff' },
];
const HINTS_TABS = [
  { glyphs: ['A' as const], label: 'Entrar' },
  { glyphs: ['B' as const], label: 'Voltar' },
];
const HINTS_PANE = [
  { glyphs: ['A' as const], label: 'Escolher' },
  { glyphs: ['B' as const], label: 'Abas' },
];

interface PaneSpec {
  heading: string;
  lead?: string;
  before?: ReactNode;
  items: MenuItemSpec[];
  after?: ReactNode;
  empty?: ReactNode;
}

/** Rola o item em foco para dentro da lista (a lista tem overflow oculto; so o foco rola). */
function reveal(focusKey: string): void {
  const el = Array.from(document.querySelectorAll<HTMLElement>('[data-focus-key]')).find((e) => e.dataset.focusKey === focusKey);
  if (el && typeof el.scrollIntoView === 'function') el.scrollIntoView({ block: 'nearest', behavior: scrollBehavior() });
}

/**
 * Configuracoes (CA-04, CA-16, CA-66, CA-67, CA-70): abas a esquerda (Tela, Biblioteca, Geral, Itens ocultos)
 * e painel a direita, duas zonas de foco. ←/→ trocam de zona; B no painel volta as abas; B nas abas sai.
 * Carregada sob demanda (chunk proprio).
 */
export default function SettingsScreen({
  active,
  entry,
  headerExtras,
}: {
  active: boolean;
  entry: ScreenEntry;
  headerExtras?: ReactNode;
}) {
  const [tab, setTab] = useState<TabId>('screen');
  const [zone, setZone] = useState<'tabs' | 'pane'>('tabs');
  const { push, remove } = useNavigation.getState();
  const settings = useSettings((s) => s.settings);
  const monitors = useSettings((s) => s.monitors);
  const hidden = useSettings((s) => s.hidden);
  const autostartFailed = useSettings((s) => s.autostartFailed);
  const games = useLibrary((s) => s.games);
  const refreshing = useLibrary((s) => s.refreshing);
  const updatePhase = useUpdate((s) => s.phase);
  const layout = useAppearance((s) => s.layout);

  useEffect(() => {
    void useSettings.getState().load();
    // Monitor ligado ou desligado com a tela aberta (CA-66, CA-67).
    return events.displayChanged((list) => {
      useSettings.getState().setMonitors(list);
      void useSettings.getState().load();
    });
  }, []);

  const pane: PaneSpec = useMemo(() => {
    const st = useSettings.getState;
    switch (tab) {
      case 'screen':
        return {
          heading: 'Monitor do launcher',
          lead: 'O SofaLauncher abre em tela cheia no monitor escolhido.',
          before: chosenMonitorMissing(settings, monitors) ? (
            <div className="callout" role="status">
              <Icon name="alert" />
              <span>
                O monitor escolhido não está conectado. O SofaLauncher está no monitor principal e volta ao monitor escolhido
                quando ele reconectar.
              </span>
            </div>
          ) : null,
          items: monitors.map((m, i) => ({
            id: `m${i}`,
            label: m.name,
            icon: 'monitor' as const,
            hint: m.primary ? 'Principal' : undefined,
            right: m.current ? (
              <span className="tagsm">
                Em uso <Icon name="check" size="s" />
              </span>
            ) : null,
            onSelect: () => void st().chooseMonitor(m),
          })),
          after: monitors.length === 1 ? <div className="sub">Só há um monitor conectado.</div> : null,
        };
      case 'appearance':
        return {
          heading: 'Layout da tela inicial',
          lead: 'Muda a disposição dos jogos e as cores do SofaLauncher. Vale na hora.',
          items: LAYOUTS.map((l) => ({
            id: `layout-${l.id}`,
            label: l.label,
            icon: l.icon,
            hint: l.hint,
            role: 'menuitemradio' as const,
            checked: layout === l.id,
            right:
              layout === l.id ? (
                <span className="tagsm">
                  Em uso <Icon name="check" size="s" />
                </span>
              ) : null,
            onSelect: () => useAppearance.getState().setLayout(l.id),
          })),
        };
      case 'library':
        return {
          heading: 'Biblioteca',
          lead: librarySummary(games),
          items: [
            {
              id: 'refresh',
              label: 'Atualizar biblioteca',
              icon: 'refresh' as const,
              hint: 'Procura jogos novos na Steam, na Epic e no GOG.',
              right: refreshing ? <div className="spinner sm" role="status" aria-label="Atualizando" /> : null,
              onSelect: () => void useLibrary.getState().refresh(),
            },
            {
              id: 'add',
              label: 'Adicionar jogo',
              icon: 'plus' as const,
              hint: 'Escolha um arquivo .exe em um disco deste PC.',
              right: <Icon name="chevron" />,
              onSelect: () => push('folderBrowser'),
            },
          ],
        };
      case 'general':
        return {
          heading: 'Geral',
          items: [
            {
              id: 'auto',
              label: 'Iniciar com o Windows',
              icon: 'power' as const,
              hint: 'Abre o SofaLauncher quando você entra no Windows. Vale só para o seu usuário.',
              role: 'switch' as const,
              checked: settings?.autostart ?? false,
              right: <Switch on={settings?.autostart ?? false} />,
              onSelect: () => void st().setAutostart(!(settings?.autostart ?? false)),
            },
            {
              id: 'update',
              label: 'Procurar atualizações',
              icon: 'download' as const,
              hint: `Você usa a versão ${APP_VERSION}. Precisa de internet.`,
              right:
                updatePhase !== 'idle' ? <div className="spinner sm" role="status" aria-label="Procurando atualizações" /> : null,
              onSelect: () => void useUpdate.getState().check(true),
            },
          ],
          after: (
            <>
              {autostartFailed ? (
                <div className="errline" role="alert">
                  <Icon name="alert" size="s" />
                  <span>Não foi possível alterar a inicialização com o Windows. Tente de novo.</span>
                </div>
              ) : null}
              {/* Texto so de leitura: mecanismo da inicializacao e pasta de logs (SettingsDto). */}
              {settings ? (
                <dl className="info">
                  <dt>Inicialização</dt>
                  <dd>{mechanismLabel(settings.autostartMechanism)}</dd>
                  <dt>Pasta de logs</dt>
                  <dd>{settings.logDir}</dd>
                </dl>
              ) : null}
              <div className="callout">
                <Icon name="gamepad" />
                <span>
                  Para ligar o PC e jogar sem teclado, configure o logon automático ou o Windows Hello no Windows. O SofaLauncher
                  começa na área de trabalho.
                </span>
              </div>
              <div className="callout">
                <Icon name="shield" />
                <span>
                  O SofaLauncher nunca roda como administrador. Jogos que exigem administrador pedem confirmação no Windows, com
                  teclado ou mouse.
                </span>
              </div>
              <div className="sub">{APP_FOOTER}</div>
            </>
          ),
        };
      case 'hidden':
        return {
          heading: 'Itens ocultos',
          lead: hidden.length ? 'Escolha um item para voltar à biblioteca.' : undefined,
          items: hidden.map((g) => ({
            id: `h:${g.id}`,
            label: g.name,
            icon: 'eyeoff' as const,
            right: <span className="tagsm">{originLabel(g.origin)}</span>,
            onSelect: () => push('confirmDialog', undefined, { confirm: 'restore', gameId: g.id }),
          })),
          empty: (
            <div className="empty-s">
              <Icon name="eyeoff" size="xl" />
              <b>Nenhum item oculto</b>
              <span>Os jogos que você ocultar aparecem aqui.</span>
            </div>
          ),
        };
    }
  }, [tab, settings, monitors, hidden, games, refreshing, autostartFailed, updatePhase, layout, push]);

  const firstKey = pane.items.find((i) => !i.disabled) ? `menu-${pane.items.find((i) => !i.disabled)!.id}` : null;
  const enterPane = () => {
    if (firstKey) setFocus(firstKey);
  };

  // ←/→ trocam de zona (abas <-> painel); ↑/↓ ficam com a navegacao espacial dentro da zona.
  useDirectionHandler((direction) => {
    const inTabs = getCurrentFocusKey().startsWith('tab-');
    if (inTabs && direction === 'right') {
      enterPane();
      return true;
    }
    if (!inTabs && direction === 'left') {
      setFocus(`tab-${tab}`);
      return true;
    }
    return false;
  }, active);
  useBackHandler(active ? () => (zone === 'pane' ? setFocus(`tab-${tab}`) : remove(entry.key)) : undefined);

  // Depois de restaurar um item o foco segue para o proximo (ou volta as abas se a lista esvaziou).
  useEffect(() => {
    if (tab !== 'hidden' || zone !== 'pane') return;
    if (!pane.items.some((i) => `menu-${i.id}` === getCurrentFocusKey())) {
      setFocus(firstKey ?? `tab-${tab}`);
    }
    // So reage a mudanca da lista de ocultos.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [hidden]);

  const items = pane.items.map((item) => ({
    ...item,
    onFocus: () => {
      setZone('pane');
      reveal(`menu-${item.id}`);
    },
  }));

  return (
    <div className="layer home screen settings" data-active={active ? 'true' : 'false'} data-screen="settings">
      <div className="spotlight" />
      <SafeArea>
        <AppHeader refreshing={refreshing}>{headerExtras}</AppHeader>
        <h1 className="title">Configurações</h1>
        <FocusLayer focusKey="settings" defaultFocus="tab-screen" active={active} rememberFocus>
          <div className="cols">
            <div className="tabs" role="tablist" aria-label="Configurações">
              {TABS.map((t) => (
                <Focusable
                  key={t.id}
                  focusKey={`tab-${t.id}`}
                  className={`tab${tab === t.id ? ' sel' : ''}`}
                  role="tab"
                  ariaSelected={tab === t.id}
                  onFocus={() => {
                    setTab(t.id);
                    setZone('tabs');
                  }}
                  onActivate={enterPane}
                >
                  <Icon name={t.icon} />
                  <span>{t.label}</span>
                  {t.id === 'hidden' && hidden.length ? <span className="cnt">{hidden.length}</span> : null}
                </Focusable>
              ))}
            </div>
            <section className="pane" aria-label={pane.heading}>
              <h2>{pane.heading}</h2>
              {pane.lead ? <div className="lead">{pane.lead}</div> : null}
              {pane.before}
              {items.length ? (
                <div className="list">
                  <MenuList label={pane.heading} items={items} />
                </div>
              ) : (
                pane.empty
              )}
              {pane.after}
            </section>
          </div>
        </FocusLayer>
        <ButtonHints hints={zone === 'tabs' ? HINTS_TABS : HINTS_PANE} />
      </SafeArea>
    </div>
  );
}
