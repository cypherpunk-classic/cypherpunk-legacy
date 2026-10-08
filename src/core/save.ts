/**
 * Persistence. Browser storage can be unavailable (private windows, sandboxed
 * previews, blocked site data), so every access is wrapped and the game keeps
 * working in memory if storage fails.
 */

const memory = new Map<string, string>();

function storage(): Storage | null {
  try {
    const s = window.localStorage;
    const probe = '__cpl_probe__';
    s.setItem(probe, '1');
    s.removeItem(probe);
    return s;
  } catch {
    return null;
  }
}

export function readKey(key: string): string | null {
  const s = storage();
  try {
    return s ? s.getItem(key) : (memory.get(key) ?? null);
  } catch {
    return memory.get(key) ?? null;
  }
}

export function writeKey(key: string, value: string): boolean {
  memory.set(key, value);
  const s = storage();
  if (!s) return false;
  try {
    s.setItem(key, value);
    return true;
  } catch {
    return false;
  }
}

export function removeKey(key: string): void {
  memory.delete(key);
  try {
    storage()?.removeItem(key);
  } catch {
    /* ignore */
  }
}

export function readJson<T>(key: string): T | null {
  const raw = readKey(key);
  if (!raw) return null;
  try {
    return JSON.parse(raw) as T;
  } catch {
    return null;
  }
}

export const writeJson = (key: string, value: unknown): boolean => writeKey(key, JSON.stringify(value));

export const storageAvailable = (): boolean => storage() !== null;
