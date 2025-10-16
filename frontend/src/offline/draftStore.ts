const STORAGE_KEY = 'fsi-expense-drafts';

export interface StoredDraft {
  id: string;
  payload: unknown;
  updatedAt: string;
}

export const loadDrafts = (): StoredDraft[] => {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    return JSON.parse(raw) as StoredDraft[];
  } catch (error) {
    console.warn('Unable to read drafts', error);
    return [];
  }
};

export const saveDrafts = (drafts: StoredDraft[]) => {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(drafts));
  } catch (error) {
    console.warn('Unable to persist drafts', error);
  }
};

export const upsertDraft = (draft: StoredDraft) => {
  const drafts = loadDrafts();
  const filtered = drafts.filter((existing) => existing.id !== draft.id);
  filtered.push(draft);
  saveDrafts(filtered);
};

export const removeDraft = (id: string) => {
  const drafts = loadDrafts().filter((draft) => draft.id !== id);
  saveDrafts(drafts);
};
