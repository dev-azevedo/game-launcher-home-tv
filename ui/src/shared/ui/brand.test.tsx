import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { BootSplash } from '@/app/BootSplash';
import { BrandLogo, BrandSymbol, BrandWordmark } from './Brand';

const html = readFileSync(resolve(process.cwd(), 'index.html'), 'utf8');

describe('marca SofaLauncher (FE-20, D-49, D-52)', () => {
  it('o <title> e o favicon vem do asset local da marca, sem URL externa', () => {
    expect(html).toContain('<title>SofaLauncher</title>');
    expect(html).toMatch(/<link rel="icon"[^>]*href="\.\.\/docs\/brand\/sofa\/assets\/icon-small\.svg"/);
    expect(html).not.toMatch(/https?:\/\//);
  });

  it('todas as variantes da marca tem alt "SofaLauncher"', () => {
    render(
      <>
        <BrandLogo />
        <BrandSymbol />
        <BrandWordmark />
      </>,
    );
    expect(screen.getAllByAltText('SofaLauncher')).toHaveLength(3);
  });

  it('a abertura mostra a arte do splash', () => {
    const { container } = render(<BootSplash />);
    expect(container.querySelector('img.splash-art')).not.toBeNull();
  });
});
