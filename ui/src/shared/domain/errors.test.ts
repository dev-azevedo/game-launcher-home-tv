import { describe, expect, it } from 'vitest';
import { errorCopy, forceCloseDenied, KNOWN_ERROR_CODES } from './errors';
import { truncateName } from './text';

const GENERIC = 'Algo deu errado no Sofá. Tente de novo.';

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
