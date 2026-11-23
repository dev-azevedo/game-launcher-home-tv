import type { AutostartMechanism, GameDto, MonitorDto, Origin, SettingsDto } from '@/shared/ipc';
import { originLabel } from '@/shared/ui/Badges';
import { version } from '../../../package.json';

const ORDER: Origin[] = ['Steam', 'Epic', 'Gog', 'Manual'];

/** "N jogos: 6 Steam, 3 Epic" (microcopy, Biblioteca). */
export function librarySummary(games: readonly GameDto[]): string {
  if (games.length === 0) return '0 jogos';
  const parts = ORDER.map((o) => [o, games.filter((g) => g.origin === o).length] as const)
    .filter(([, n]) => n > 0)
    .map(([o, n]) => `${n} ${originLabel(o)}`);
  return `${games.length} ${games.length === 1 ? 'jogo' : 'jogos'}: ${parts.join(', ')}`;
}

/** O monitor escolhido nao esta conectado (CA-67): a preferencia fica, o Sofa usa o principal. */
export function chosenMonitorMissing(settings: SettingsDto | null, monitors: readonly MonitorDto[]): boolean {
  return settings?.monitorId != null && !monitors.some((m) => m.id === settings.monitorId);
}

const MECHANISM: Record<AutostartMechanism, string> = {
  Task: 'Tarefa agendada do Windows',
  Run: 'Entrada de inicialização do seu usuário',
  None: 'Desligada',
};

/** Texto do mecanismo de inicializacao (so leitura, sem acao). */
export const mechanismLabel = (m: AutostartMechanism): string => MECHANISM[m];

/** Versao do app: a do `package.json`, travada igual a do `tauri.conf.json` (tests/command_list.rs). */
export const APP_VERSION: string = version;

export const APP_FOOTER = `SofaLauncher ${APP_VERSION} · Fontes Sora e Figtree sob licença OFL`;
