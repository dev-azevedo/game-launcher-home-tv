import { act, render } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { CursorAutoHide } from './CursorAutoHide';

describe('CursorAutoHide (CA-03)', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('esconde o cursor apos 3 s e volta ao mexer o mouse', () => {
    render(<CursorAutoHide />);
    const root = document.documentElement;
    expect(root.classList.contains('nocursor')).toBe(false);
    act(() => void vi.advanceTimersByTime(2999));
    expect(root.classList.contains('nocursor')).toBe(false);
    act(() => void vi.advanceTimersByTime(1));
    expect(root.classList.contains('nocursor')).toBe(true);
    act(() => void window.dispatchEvent(new MouseEvent('mousemove')));
    expect(root.classList.contains('nocursor')).toBe(false);
    act(() => void vi.advanceTimersByTime(3000));
    expect(root.classList.contains('nocursor')).toBe(true);
  });
});
