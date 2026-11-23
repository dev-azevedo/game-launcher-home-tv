import { describe, expect, it } from 'vitest';
import { CONTRACT_ERROR_CODES } from '@/shared/ipc/types';
import { errorCopy, errorCopyFor, forceCloseDenied, KNOWN_ERROR_CODES } from './errors';
import { truncateName } from './text';

const GENERIC = 'Algo deu errado no SofaLauncher. Tente de novo.';

describe('texto por codigo de erro', () => {
  it('todo codigo conhecido tem mensagem nao vazia', () => {
    expect(KNOWN_ERROR_CODES.length).toBeGreaterThan(20);
    for (const code of KNOWN_ERROR_CODES) expect(errorCopy(code).message.length).toBeGreaterThan(0);
  });

  it('PathNotAllowed e NetworkPathBlocked tem mensagens distintas (CA-71, CA-72)', () => {
    const generic = errorCopy('PathNotAllowed').message;
    const network = errorCopy('NetworkPathBlocked');
    expect(network.message).toBe(
      'Jogos em pastas de rede não são aceitos. Copie o jogo para um disco deste PC.',
    );
    expect(network.message).not.toBe(generic);
    expect(network.actions).toEqual(['Voltar', 'Ocultar']);
  });

  it('opcao segura vem primeiro nos botoes (foco inicial)', () => {
    expect(errorCopy('SessionAlreadyActive').actions[0]).toBe('Cancelar');
    expect(errorCopy('NeedsElevation').actions[0]).toBe('Continuar');
    expect(errorCopy('GameFileMissing').actions[0]).toBe('Voltar');
  });

  it('codigo desconhecido cai no texto generico, sem repetir o codigo', () => {
    const copy = errorCopy('CodigoNovoDoNucleo');
    expect(copy.message).toBe(GENERIC);
    expect(copy.message).not.toContain('CodigoNovoDoNucleo');
  });

  it('nao confunde codigo com propriedade herdada do Object', () => {
    expect(errorCopy('constructor').message).toBe(GENERIC);
    expect(errorCopy('__proto__').message).toBe(GENERIC);
  });

  it('interpola o nome do jogo como texto puro', () => {
    const name = '<img src=x onerror=alert(1)>';
    expect(errorCopy('NeedsElevation', { gameName: name }).message).toContain(name);
    expect(forceCloseDenied({ gameName: 'Minério' }).message).toContain('Minério');
  });
});

describe('cobertura do contrato (docs/api secao 5)', () => {
  it('os 28 codigos do contrato tem texto proprio, sem cair em "Desconhecido" por omissao', () => {
    expect(CONTRACT_ERROR_CODES).toHaveLength(28);
    for (const code of CONTRACT_ERROR_CODES) expect(KNOWN_ERROR_CODES).toContain(code);
  });

  it('o catalogo nao tem codigo fora do contrato', () => {
    for (const code of KNOWN_ERROR_CODES) expect(CONTRACT_ERROR_CODES as readonly string[]).toContain(code);
  });

  it('ElevationDenied (CA-73) usa o nome do jogo e so oferece Voltar', () => {
    const copy = errorCopy('ElevationDenied', { gameName: 'Minério' });
    expect(copy.message).toBe('O Windows não deu permissão para abrir Minério.');
    expect(copy.actions).toEqual(['Voltar']);
  });

  it('InvalidRequest e Internal mostram o texto generico, sem detalhe tecnico', () => {
    expect(errorCopy('InvalidRequest').message).toBe(GENERIC);
    expect(errorCopy('Internal').message).toBe(GENERIC);
  });

  it('StoreNotInstalled traduz params.origin para o nome da loja (D4)', () => {
    expect(errorCopyFor({ code: 'StoreNotInstalled', params: { origin: 'Epic' } }).message).toContain('a Epic neste PC');
    expect(errorCopyFor({ code: 'StoreNotInstalled', params: { origin: 'Gog' } }).message).toContain('a GOG neste PC');
    expect(errorCopy('StoreNotInstalled').message).toContain('a loja neste PC');
  });

  it('GameFileMissing mostra o nome do jogo, nunca um caminho (D3)', () => {
    const copy = errorCopy('GameFileMissing', { gameName: 'Bruma' });
    expect(copy.support).toBe('Bruma');
    expect(copy.actions).toEqual(['Voltar', 'Ocultar']);
  });

  it('AccessDenied tem dois textos conforme o comando (D5)', () => {
    const folder = errorCopy('AccessDenied', { command: 'fs_list_dir' });
    const force = errorCopy('AccessDenied', { command: 'session_force_close', gameName: 'Minério' });
    expect(folder.message).toBe('Sem permissão para abrir esta pasta.');
    expect(force.message).toContain('O jogo roda como administrador');
    expect(force.message).toContain('Minério');
    expect(errorCopy('AccessDenied').message).toBe(folder.message);
  });
});

describe('truncateName (E-13)', () => {
  it('nome curto fica igual', () => expect(truncateName('Minério', 20)).toBe('Minério'));
  it('nome longo termina em reticencias dentro do limite', () => {
    const out = truncateName('A'.repeat(50), 10);
    expect(Array.from(out)).toHaveLength(10);
    expect(out.endsWith('…')).toBe(true);
  });
  it('nao parte emoji ao meio', () => {
    expect(truncateName('🎮🎮🎮🎮🎮', 3)).toBe('🎮🎮…');
  });
});
