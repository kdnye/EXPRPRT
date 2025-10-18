import { Dispatch, SetStateAction, useCallback, useEffect, useRef, useState } from 'react';
import { loadDrafts, removeDraft, upsertDraft } from '../offline/draftStore';

/**
 * Persists expense form state to `localStorage` with debounce semantics for
 * offline support.
 *
 * The hook mirrors the draft persistence strategy described in
 * `docs/architecture.md` (offline UX) by writing to the shared
 * `draftStore`. Each state change is debounced (default 400 ms) before
 * calling `upsertDraft`, minimizing synchronous storage writes while keeping
 * data resilient during intermittent connectivity. Storage is skipped when
 * `window.localStorage` is unavailable (server rendering, private mode, or
 * quota exhaustion), and consumers are expected to call the returned
 * `resetDraft` helper after submission so `removeDraft` can prune stale drafts.
 *
 * Because the browser-only storage quota is limited (~5–10 MB depending on
 * the engine) and not encrypted, callers should keep payloads small and avoid
 * attaching PII beyond what is necessary for drafts. The hook falls back to
 * the provided `createInitialDraft` factory when no persisted state exists or
 * after cleanup.
 */

interface UseExpenseDraftOptions {
  debounceMs?: number;
}

type DraftFactory<T> = () => T;

const canUseStorage = () => typeof window !== 'undefined' && typeof window.localStorage !== 'undefined';

const withPersistedDraft = <T extends Record<string, unknown>>(
  draftId: string,
  createInitialDraft: DraftFactory<T>
) => {
  const baseDraft = createInitialDraft();

  if (!canUseStorage()) {
    return baseDraft;
  }

  const persisted = loadDrafts().find((entry) => entry.id === draftId);
  if (persisted && typeof persisted.payload === 'object' && persisted.payload !== null) {
    return { ...baseDraft, ...(persisted.payload as Partial<T>) };
  }

  return baseDraft;
};

export const useExpenseDraft = <T extends Record<string, unknown>>(
  draftId: string,
  createInitialDraft: DraftFactory<T>,
  options: UseExpenseDraftOptions = {}
): [T, Dispatch<SetStateAction<T>>, () => void] => {
  const { debounceMs = 400 } = options;

  const [draft, setDraft] = useState<T>(() => withPersistedDraft(draftId, createInitialDraft));
  const skipNextPersistRef = useRef(false);

  useEffect(() => {
    if (!canUseStorage()) {
      return;
    }

    if (skipNextPersistRef.current) {
      skipNextPersistRef.current = false;
      return;
    }

    const handle = window.setTimeout(() => {
      upsertDraft({
        id: draftId,
        payload: draft,
        updatedAt: new Date().toISOString()
      });
    }, debounceMs);

    return () => {
      window.clearTimeout(handle);
    };
  }, [draft, draftId, debounceMs]);

  const resetDraft = useCallback(() => {
    skipNextPersistRef.current = true;
    setDraft(createInitialDraft());
    if (canUseStorage()) {
      removeDraft(draftId);
    }
  }, [createInitialDraft, draftId]);

  return [draft, setDraft, resetDraft];
};

export default useExpenseDraft;
