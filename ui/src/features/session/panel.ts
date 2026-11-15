import { gameName } from '@/features/library/domain';
import type { GameDto, SessionDto, SessionState } from '@/shared/ipc';

/** Estados em que ha um jogo aberto (ou abrindo/fechando) e o PC nao deve ser desligado sem aviso (CA-48). */
const OPEN_STATES: readonly SessionState[] = ['Launching', 'Running', 'Ending', 'Closing', 'CloseOverdue'];

/** Nome do jogo da sessao ativa, ou `null` sem jogo aberto. Resolvido pela biblioteca (SEC-06). */
export function openGameName(session: SessionDto, games: readonly GameDto[]): string | null {
  if (!session.gameId || !OPEN_STATES.includes(session.state)) return null;
  return gameName(games, session.gameId, 'O jogo');
}

/** "Jogando agora" / "Jogando ha N min" / "Jogando ha H h M min" (microcopy, painel do jogo). */
export function playingFor(since: number | null, now: number): string {
  if (since === null) return 'Jogando agora';
  const minutes = Math.max(0, Math.floor((now - since) / 60000));
  if (minutes < 1) return 'Jogando agora';
  if (minutes < 60) return `Jogando há ${minutes} min`;
  return `Jogando há ${Math.floor(minutes / 60)} h ${minutes % 60} min`;
}
