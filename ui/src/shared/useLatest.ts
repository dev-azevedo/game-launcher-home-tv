import { useLayoutEffect, useRef, type RefObject } from 'react';

/** Guarda o valor mais recente para callbacks de longa vida sem reassinar efeitos. */
export function useLatest<T>(value: T): RefObject<T> {
  const ref = useRef(value);
  useLayoutEffect(() => {
    ref.current = value;
  });
  return ref;
}
