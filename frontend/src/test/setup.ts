import '@testing-library/jest-dom/vitest';

// Polyfill crypto.randomUUID for jsdom
if (typeof globalThis.crypto?.randomUUID !== 'function') {
  globalThis.crypto = {
    ...globalThis.crypto,
    randomUUID: () => `test-${Math.random().toString(16).slice(2)}`
  } as Crypto;
}
