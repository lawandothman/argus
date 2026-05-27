"use client";

import { useEffect, useState } from "react";

/** Poll an async function on an interval, returning the latest value or error. */
export function usePoll<T>(fn: () => Promise<T>, intervalMs = 3000) {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    const tick = () =>
      fn()
        .then((value) => {
          if (alive) {
            setData(value);
            setError(null);
          }
        })
        .catch((err) => alive && setError(String(err)));
    tick();
    const id = setInterval(tick, intervalMs);
    return () => {
      alive = false;
      clearInterval(id);
    };
  }, [fn, intervalMs]);

  return { data, error };
}
