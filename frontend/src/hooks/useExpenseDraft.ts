import { Dispatch, SetStateAction, useCallback, useEffect, useState } from 'react';
import { loadDrafts, removeDraft, upsertDraft } from '../offline/draftStore';

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

  useEffect(() => {
    if (!canUseStorage()) {
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
    setDraft(createInitialDraft());
    if (canUseStorage()) {
      removeDraft(draftId);
    }
  }, [createInitialDraft, draftId]);

  return [draft, setDraft, resetDraft];
};

export default useExpenseDraft;
