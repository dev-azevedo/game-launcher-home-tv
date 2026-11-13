import { ESLint } from 'eslint';
import { describe, expect, it } from 'vitest';

async function lint(code: string, filePath: string) {
  const eslint = new ESLint({ cwd: process.cwd() });
  const [result] = await eslint.lintText(code, { filePath });
  return result!.messages.map((m) => m.ruleId);
}

describe('regras de seguranca do ESLint (SEC-07, INV-02)', () => {
  it('acusa dangerouslySetInnerHTML', async () => {
    const rules = await lint(
      'export const X = ({ h }: { h: string }) => <div dangerouslySetInnerHTML={{ __html: h }} />;',
      'src/features/x/X.tsx',
    );
    expect(rules).toContain('react/no-danger');
  });

  it('acusa innerHTML e insertAdjacentHTML', async () => {
    const rules = await lint(
      'export function f(el: HTMLElement, h: string) { el.innerHTML = h; el.insertAdjacentHTML("beforeend", h); }',
      'src/features/x/f.ts',
    );
    expect(rules).toContain('no-restricted-properties');
    expect(rules).toContain('no-restricted-syntax');
  });

  it('acusa import de @tauri-apps fora de shared/ipc e libera dentro', async () => {
    const code = "import { invoke } from '@tauri-apps/api/core';\nexport const f = () => invoke;";
    expect(await lint(code, 'src/features/x/f.ts')).toContain('no-restricted-imports');
    expect(await lint(code, 'src/shared/ipc/f.ts')).not.toContain('no-restricted-imports');
  });
});
