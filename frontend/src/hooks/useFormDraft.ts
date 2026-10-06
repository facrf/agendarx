import { useEffect, useRef, useState } from "react";
import { useAuth } from "../contexts/AuthContext";

interface StoredDraft<T> { version: number; updated: number; value: T }
function sameShape(value: unknown, sample: unknown): boolean {
  if (sample === null) return value === null || ["number", "string", "boolean"].includes(typeof value);
  if (Array.isArray(sample)) return Array.isArray(value);
  if (typeof sample === "object") return value !== null && typeof value === "object"
    && Object.entries(sample as Record<string, unknown>).every(([key, item]) => sameShape((value as Record<string, unknown>)[key], item));
  return typeof value === typeof sample;
}
export function useFormDraft<T>({ name, value, enabled = true, meaningful = true, restore, validate }: {
  name: string; value: T; enabled?: boolean; meaningful?: boolean; restore: (value: T) => void; validate?: (value: T) => boolean;
}) {
  const { usuario } = useAuth();
  const key = `agendarx:draft:v1:${usuario?.id}:${name}`;
  const [pending, setPending] = useState<StoredDraft<T> | null>(null);
  const [savedAt, setSavedAt] = useState<number | null>(null);
  const [unavailable, setUnavailable] = useState(false);
  const baseline = useRef("");
  const activeKey = useRef("");
  const serialized = JSON.stringify(value);
  useEffect(() => {
    if (!enabled || !usuario) { activeKey.current = ""; return; }
    if (activeKey.current !== key) {
      activeKey.current = key; baseline.current = serialized;
      setPending(null); setSavedAt(null);
      try {
        const raw = localStorage.getItem(key);
        let item: StoredDraft<T> | null = null;
        try { item = raw ? JSON.parse(raw) as StoredDraft<T> : null; } catch { /* Invalid local data is discarded below. */ }
        let valid = false;
        try { valid = Boolean(item && item.version === 1 && Number.isFinite(item.updated) && Date.now() - item.updated < 30 * 86400_000 && sameShape(item.value, value) && (!validate || validate(item.value))); } catch { /* Invalid field shapes are not restored. */ }
        setUnavailable(false);
        if (item && valid) {
          setPending(item);
          return;
        }
        if (raw) localStorage.removeItem(key);
      } catch { setUnavailable(true); }
    }
    if (pending || baseline.current === serialized) return;
    if (!meaningful) {
      try { localStorage.removeItem(key); setSavedAt(null); baseline.current = serialized; }
      catch { setUnavailable(true); }
      return;
    }
    try {
      const updated = Date.now();
      localStorage.setItem(key, JSON.stringify({ version: 1, updated, value }));
      setSavedAt(updated); baseline.current = serialized; setUnavailable(false);
    } catch { setUnavailable(true); }
  }, [key, enabled, usuario, serialized, value, meaningful, pending, validate]);
  const clear = () => {
    try { localStorage.removeItem(key); } catch { setUnavailable(true); }
    setPending(null); setSavedAt(null); baseline.current = serialized;
  };
  return { pending, savedAt, unavailable, clear,
    recover: () => { if (pending) { restore(pending.value); setPending(null); setSavedAt(pending.updated); } },
  };
}
