# Freight Services Expense Portal

A full-stack implementation of the Freight Services expense workflow using Rust (Axum), React + Vite, and PostgreSQL. Employees capture expenses with receipts, managers approve within a shared queue, and finance finalizes batches for NetSuite export while policy controls run end-to-end.

> **Note for contributors:** AI agents and humans alike must avoid generating binary files in this repository. Stick to source code, configuration, and textual assets.

## Repository Structure

- `backend/` – Axum API, domain services, SQLx migrations, background jobs
- `frontend/` – React single-page application with employee, manager, and finance consoles
- `docs/` – Architecture reference material

## Key Capabilities

- Policy-driven validation for per-diem, mileage, and travel class before manager review
- Chunked receipt uploads backed by a pluggable storage provider (local filesystem, S3/GCS-ready interface)
- Manager and finance workflows with optimistic locking and tamper-resistant audit logging
- NetSuite batch export stubs ready for credential wiring plus retry-aware job scaffolding
- Offline-aware React UI with local draft persistence and service worker caching

## Getting Started

### Prerequisites

- Rust 1.74+
- Node.js 20+
- Docker (optional, for containerized development)
- PostgreSQL 15 (local or via Docker Compose)

Install JavaScript dependencies locally after cloning so `node_modules` directories stay on your machine:

```bash
npm install       # project-level tooling
cd frontend && npm install
```

Substitute `npm ci` for deterministic installs in CI or reproducible local setups.

### Environment Configuration

Copy the sample configuration and adjust as needed:

```bash
cp .env.example .env
```

All backend settings use the `EXPENSES__` prefix and are parsed by `backend/src/infrastructure/config.rs`. Frontend builds read `VITE_` variables at compile time and defer to runtime overrides via HTML meta tags or `window.__FSI_EXPENSES_CONFIG__`.

### Run Everything with Docker Compose

```bash
docker compose up --build
```

Services exposed:

- API: <http://localhost:8080>
- Frontend: <http://localhost:3000>
- PostgreSQL: `localhost:5432` (credentials `expenses / expenses` by default)
- Receipts uploaded during development are written to the `receipts` named volume

### Local Backend Workflow

```bash
cd backend
cargo fmt
cargo check
cargo run --bin migrator  # apply database migrations before starting the API
cargo run
```

The API listens on the host/port defined in configuration (defaults to `0.0.0.0:8080`). SQLx migrations live under `backend/migrations` and must be applied manually (for example with `cargo run --bin migrator` or `cargo sqlx migrate run`) before launching the API.

### Local Frontend Workflow

```bash
cd frontend
npm install
npm run dev
```

Visit the printed URL (typically <http://localhost:5173>) to access the SPA. The dev server proxies API calls to `VITE_API_BASE` (default `/api`).

## Testing & Quality Gates

- `cargo fmt` / `cargo check` / `cargo test` for the Rust backend
- `npm run lint` / `npm run typecheck` / `npm run test` for the React client
- CI (recommended) should run formatters, linters, unit tests, and integration tests against an ephemeral PostgreSQL instance

## Deployment Notes

- Backend Docker image defined in `backend/Dockerfile` (multi-stage Rust build)
- Frontend Docker image defined in `frontend/Dockerfile` (Node build + NGINX static host)
- Environment variables mirror `.env.example` and should be provided via secrets management in production
- NetSuite integration is stubbed; replace `infrastructure/netsuite.rs` with a signed REST/SOAP client once credentials are available

## Additional Documentation

- `docs/architecture.md` – Policy mapping, data model, and workflow details that guided this implementation
- `POLICY.md` – Source policy document for expense categories, limits, and approval hierarchy
- Contributions should include automated tests, documentation updates, and respect for PII/data-safety guidance in `AGENTS.md`
