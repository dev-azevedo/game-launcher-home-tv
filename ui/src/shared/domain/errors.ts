/**
 * Texto pt-BR por codigo de erro (catalogo: docs/design/launcher-tv/microcopy.md, secao 2).
 * A UI traduz o `code` e nunca mostra texto vindo do nucleo (SEC-06). Parametros (`gameId`, `path`)
 * so resolvem o nome do jogo pela biblioteca. Codigo fora da tabela cai em "Desconhecido".
 *
 * Parte independente do contrato: a lista parte da arquitetura 4.1 + `NetworkPathBlocked` (CA-71).
 * FE-02 completo (100% dos codigos de docs/api/) depende do BE-07.
 */
export type ErrorContext = {
  /** Nome do jogo ja resolvido pela biblioteca. */
  gameName?: string;
  /** Nome do jogo aberto (troca de jogo). */
  currentGameName?: string;
  /** Loja, para `StoreNotInstalled`. */
  storeName?: string;
};

export interface ErrorCopy {
  message: string;
  /** Linha de apoio opcional. */
  support?: string;
  /** Rotulos dos botoes; o primeiro e o foco inicial (opcao segura). */
  actions: readonly string[];
}

type CopyFactory = (ctx: ErrorContext) => ErrorCopy;

const name = (ctx: ErrorContext) => ctx.gameName ?? 'o jogo';

const CATALOG: Record<string, CopyFactory> = {
  Busy: () => ({
    message: 'Um momento. O Sofá ainda está terminando outra ação. Tente de novo em instantes.',
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
    message: 'Este local não é aceito. O Sofá só abre jogos de discos deste PC.',
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
  AccessDenied: () => ({
    message: 'Sem permissão para abrir esta pasta.',
    support: 'O Windows não deixa o Sofá ver o que há aqui.',
    actions: ['Voltar'],
  }),
  EmptyName: () => ({ message: 'O nome não pode ficar vazio.', actions: [] }),
  UnknownGame: () => ({ message: 'Este jogo não está mais na biblioteca.', actions: ['Voltar'] }),
  SessionAlreadyActive: (ctx) => ({
    message: `${ctx.currentGameName ?? 'Outro jogo'} está aberto. Fechar e abrir ${name(ctx)}?`,
    actions: ['Cancelar', 'Fechar e abrir'],
  }),
  GameFileMissing: () => ({
    message: 'Não encontramos o arquivo do jogo.',
    actions: ['Voltar', 'Ocultar'],
  }),
  StoreNotInstalled: (ctx) => ({
    message: `Não encontramos a ${ctx.storeName ?? 'loja'} neste PC. Instale a loja para abrir este jogo.`,
    actions: ['Voltar'],
  }),
  NeedsElevation: (ctx) => ({
    message: `${ctx.gameName ?? 'O jogo'} precisa de permissão de administrador para abrir. O Windows vai pedir a sua confirmação.`,
    actions: ['Continuar', 'Voltar'],
  }),
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

const UNKNOWN: ErrorCopy = { message: 'Algo deu errado no Sofá. Tente de novo.', actions: ['Voltar'] };

/** `AccessDenied` ao forcar o fechamento tem texto proprio (jogo elevado). */
export function forceCloseDenied(ctx: ErrorContext): ErrorCopy {
  return {
    message: `Não foi possível fechar ${name(ctx)}. O jogo roda como administrador. Feche pelo menu do próprio jogo.`,
    actions: ['Voltar'],
  };
}

export function errorCopy(code: string, ctx: ErrorContext = {}): ErrorCopy {
  const factory = Object.hasOwn(CATALOG, code) ? CATALOG[code] : undefined;
  return (factory ?? (() => UNKNOWN))(ctx);
}

export const KNOWN_ERROR_CODES: readonly string[] = Object.keys(CATALOG);
