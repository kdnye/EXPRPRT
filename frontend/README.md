# FSI Expense Portal Frontend

This package provides the React single-page application for the Freight Services expense workflow. It includes dedicated portals for employees, managers, and finance users with offline-friendly behaviour and policy-aware messaging.

## Scripts

- `npm run dev` – Start the Vite development server with hot module replacement.
- `npm run build` – Type-check the project and build an optimized production bundle.
- `npm run preview` – Preview the production build locally.
- `npm run lint` – Run ESLint against all TypeScript/React files.
- `npm run typecheck` – Run the TypeScript compiler without emitting artifacts.
- `npm run test` – Execute unit tests with Vitest (add specs under `src/`).

## Environment variables

Configure the API base URL by setting `VITE_API_BASE` in your `.env` file. At runtime the application also looks for either `window.__FSI_EXPENSES_CONFIG__` or the `fsi-expenses-api-base` meta tag so deployments can override the target domain without rebuilding. When the backend is running in authentication bypass mode, set `VITE_AUTH_BYPASS=true` (or provide `window.__FSI_EXPENSES_CONFIG__.authBypass`) so the shell mirrors the impersonated role and renders the workspace without prompting for credentials.

## Offline support

A lightweight service worker precaches the application shell and serves cached assets when the device goes offline. Draft expense data is saved to `localStorage` via the `offline/draftStore.ts` helper.
