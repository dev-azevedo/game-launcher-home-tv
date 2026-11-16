import type { ErrorDto, Origin } from '@/shared/ipc/types';

/**
 * Texto pt-BR por codigo de erro (catalogo: docs/design/launcher-tv/microcopy.md, secao 2),
 * cobrindo os 26 codigos de docs/api/launcher-tv.md. A UI traduz o `code` e nunca mostra texto
 * vindo do nucleo (SEC-06). Os `params` (`gameId`, `origin`) so resolvem o nome do jogo (pela
 * biblioteca) e o nome da loja. Codigo fora da tabela cai em "Desconhecido".
 */
export type ErrorContext = {
  /** Nome do jogo ja resolvido pela biblioteca a partir de `params.gameId`. */
  gameName?: string;
  /** Nome do jogo aberto (troca de jogo). */
  currentGameName?: string;
  /** `params.origin` de `StoreNotInstalled`. */
  origin?: Origin;
  /** Comando que falhou, para codigos com dois textos (`AccessDenied`: pasta ou forcar fechamento). */
  command?: 'session_force_close' | 'fs_list_dir';
};

const STORE_NAME: Record<Origin, string> = { Steam: 'Steam', Epic: 'Epic', Gog: 'GOG', Manual: 'loja' };

export interface ErrorCopy {
  message: string;
  /** Linha de apoio opcional. */
  support?: string;
  /** Rotulos dos botoes; o primeiro e o foco inicial (opcao segura). */
  actions: readonly string[];
}

type CopyFactory = (ctx: ErrorContext) => ErrorCopy;

const name = (ctx: ErrorContext) => ctx.gameName ?? 'o jogo';

const UNKNOWN: ErrorCopy = { message: 'Algo deu errado no SofaLauncher. Tente de novo.', actions: ['Voltar'] };

const CATALOG: Record<string, CopyFactory> = {
  Busy: () => ({
    message: 'Um momento. O SofaLauncher ainda está terminando outra ação. Tente de novo em instantes.',
    actions: ['Voltar'],
  }),
  Duplicate: () => ({
    message: 'Este jogo já está na biblioteca.',
    support: 'Não criamos uma segunda cópia.',
    actions: ['Voltar'],
  }),
  NotAnExecutable: () => ({
    message: 'Este arquivo não é um executável válido.',
    support:
      'O arquivo tem a extensão .exe, mas o Windows não o reconhece como programa. Escolha outro arquivo.',
    actions: ['Voltar'],
  }),
  // Cadastro e navegador de pastas (CA-72). Inicio de jogo em rede usa NetworkPathBlocked (CA-71).
  PathNotAllowed: () => ({
    message: 'Este local não é aceito. O SofaLauncher só abre jogos de discos deste PC.',
    actions: ['Voltar'],
  }),
  NetworkPathBlocked: () => ({
    message: 'Jogos em pastas de rede não são aceitos. Copie o jogo para um disco deste PC.',
    actions: ['Voltar', 'Ocultar'],
  }),
  NotFound: () => ({
    message: 'Não encontramos esta pasta.',
    support: 'Ela pode ter sido removida, ou o disco foi desconectado.',
    actions: ['Voltar'],
  }),
  // Dois textos pelo mesmo codigo (contrato D5): forcar fechamento de jogo elevado ou pasta sem leitura.
  AccessDenied: (ctx) =>
    ctx.command === 'session_force_close'
      ? forceCloseDenied(ctx)
      : {
          message: 'Sem permissão para abrir esta pasta.',
          support: 'O Windows não deixa o SofaLauncher ver o que há aqui.',
          actions: ['Voltar'],
        },
  EmptyName: () => ({ message: 'O nome não pode ficar vazio.', actions: [] }),
  UnknownGame: () => ({ message: 'Este jogo não está mais na biblioteca.', actions: ['Voltar'] }),
  SessionAlreadyActive: (ctx) => ({
    message: `${ctx.currentGameName ?? 'Outro jogo'} está aberto. Fechar e abrir ${name(ctx)}?`,
    actions: ['Cancelar', 'Fechar e abrir'],
  }),
  // O caminho nunca cruza a IPC (SEC-03): a UI mostra so o nome do jogo, pela biblioteca (`params.gameId`).
  GameFileMissing: (ctx) => ({
    message: 'Não encontramos o arquivo do jogo.',
    support: ctx.gameName,
    actions: ['Voltar', 'Ocultar'],
  }),
  StoreNotInstalled: (ctx) => ({
    message: `Não encontramos a ${ctx.origin ? STORE_NAME[ctx.origin] : 'loja'} neste PC. Instale a loja para abrir este jogo.`,
    actions: ['Voltar'],
  }),
  NeedsElevation: (ctx) => ({
    message: `${ctx.gameName ?? 'O jogo'} precisa de permissão de administrador para abrir. O Windows vai pedir a sua confirmação.`,
    actions: ['Continuar', 'Voltar'],
  }),
  ElevationDenied: (ctx) => ({
    message: `O Windows não deu permissão para abrir ${name(ctx)}.`,
    actions: ['Voltar'],
  }),
  InvalidRequest: () => ({ ...UNKNOWN }),
  Internal: () => ({ ...UNKNOWN }),
  InvalidState: () => ({
    message: 'Isso não pode ser feito agora. Volte e tente de novo.',
    actions: ['Voltar'],
  }),
  NoActiveSession: () => ({ message: 'Nenhum jogo está aberto.', actions: ['Voltar'] }),
  ForegroundDenied: () => ({
    message: 'O Windows não deixou voltar ao jogo. Tente de novo.',
    actions: ['Tentar de novo', 'Ir para a biblioteca'],
  }),
  NotOverdue: () => ({
    message: 'O jogo ainda está fechando. Espere alguns segundos.',
    actions: ['Voltar'],
  }),
  NoAudioDevice: () => ({ message: 'Nenhum dispositivo de áudio encontrado', actions: [] }),
  Unavailable: () => ({ message: 'Esta ação não está disponível neste PC.', actions: [] }),
  TicketInvalid: () => ({
    message: 'A confirmação expirou. Escolha a ação de novo.',
    actions: ['Voltar'],
  }),
  Rejected: () => ({
    message: 'O Windows não aceitou o pedido. Tente de novo.',
    actions: ['Voltar'],
  }),
  AutostartFailed: () => ({
    message: 'Não foi possível alterar a inicialização com o Windows. Tente de novo.',
    actions: [],
  }),
  UnknownMonitor: () => ({
    message: 'Esse monitor não está mais conectado. Escolha outro.',
    actions: [],
  }),
};

/** `AccessDenied` ao forcar o fechamento tem texto proprio (jogo elevado). */
export function forceCloseDenied(ctx: ErrorContext): ErrorCopy {
  return {
    message: `Não foi possível fechar ${name(ctx)}. O jogo roda como administrador. Feche pelo menu do próprio jogo.`,
    actions: ['Voltar'],
  };
}

/** Atalho: `ErrorDto` do contrato + contexto -> texto. `params.origin` entra sozinho no contexto. */
export function errorCopyFor(error: ErrorDto, ctx: ErrorContext = {}): ErrorCopy {
  return errorCopy(error.code, { origin: error.params?.origin, ...ctx });
}

export function errorCopy(code: string, ctx: ErrorContext = {}): ErrorCopy {
  const factory = Object.hasOwn(CATALOG, code) ? CATALOG[code] : undefined;
  return (factory ?? (() => UNKNOWN))(ctx);
}

export const KNOWN_ERROR_CODES: readonly string[] = Object.keys(CATALOG);
