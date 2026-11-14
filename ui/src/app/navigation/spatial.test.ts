import { SpatialNavigation } from '@noriginmedia/norigin-spatial-navigation';
import { beforeEach, describe, expect, it } from 'vitest';
import { resetSpatialNavigation } from './spatial';

describe('navegacao espacial', () => {
  beforeEach(() => resetSpatialNavigation());

  it('as teclas nativas do norigin ficam desligadas: seta e Enter so chegam por intencao (sem passo duplo)', () => {
    // `setKeyMap` mescla com o padrao; sem desligar, cada seta moveria o foco duas vezes.
    const map = SpatialNavigation.getKeyMap();
    for (const key of ['left', 'right', 'up', 'down', 'enter'] as const) {
      expect(map[key]).toEqual([-1]);
    }
  });
});
