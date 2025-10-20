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

Install dependencies and prepare the database with the shared bootstrap script (it is invoked automatically inside our devcontainer/Codespaces image):

> **Need Rust?** Install it via [rustup](https://rustup.rs/) before running the bootstrap script so `cargo` is available on your `PATH`:
> ```bash
> curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
> ```
> Restart your shell (or source `$HOME/.cargo/env`) afterward so subsequent commands can find the toolchain.

```bash
./scripts/bootstrap.sh
```

The script installs JavaScript dependencies, launches the PostgreSQL service defined in `compose.yaml`, waits for readiness, and runs the Rust migrator (`cargo run --bin migrator`) so the schema exists before starting the API.

Once bootstrapped, start both the Axum API and the Vite dev server with a single command:

```bash
./scripts/dev-start.sh
```

The helper keeps the backend and frontend running side by side (Ctrl+C stops both). Because the devcontainer executes the same script on attach, local development and GitHub Codespaces share an identical “one command” workflow.

### Syncing an Existing Checkout

When you are returning to an existing workstation (for example, a local WSL install) run the following to pull the latest code, keep your `.env`, refresh dependencies, and restart the dev servers:

```bash
cd ~/EXPRPRT
git fetch --all --prune
git switch main
git reset --hard origin/main
[ -f .env ] || cp .env.example .env
./scripts/bootstrap.sh
./scripts/dev-start.sh
```

The bootstrap step reinstalls backend and frontend dependencies, applies database migrations, and ensures Dockerized services such as PostgreSQL are running. The `dev-start.sh` helper then brings up both the Axum API and the Vite frontend so you can resume development immediately.

### Environment Configuration

Copy the sample configuration and adjust as needed:

```bash
cp .env.example .env
```

All backend settings use the `EXPENSES__` prefix and are parsed by `backend/src/infrastructure/config.rs`. Frontend builds read `VITE_` variables at compile time and defer to runtime overrides via HTML meta tags or `window.__FSI_EXPENSES_CONFIG__`.

Key authentication variables:

- `EXPENSES__AUTH__JWT_SECRET` – symmetric secret used to sign issued JWTs.
- `EXPENSES__AUTH__DEVELOPER_CREDENTIAL` – shared developer credential accepted by `POST /api/auth/login` for local usage.

### Run Everything with Docker Compose

```bash
docker compose up --build
```

Services exposed:

- API: <http://localhost:8080>
- Frontend: <http://localhost:4173>
- PostgreSQL: `localhost:5432` (credentials `expenses / expenses` by default)
- Receipts uploaded during development are written to the `receipts` named volume

If port `5432` is already bound on your machine, set `POSTGRES_HOST_PORT` in `.env`
before running Compose (for example `POSTGRES_HOST_PORT=55432`). Likewise, override
`FRONTEND_HOST_PORT` to remap the NGINX container to a free host port when the default
`4173` is already taken (for example `FRONTEND_HOST_PORT=4300`).

### Local Backend Workflow

You can still run the backend tooling piecemeal when needed:

```bash
cd backend
cargo fmt
cargo check
cargo run --bin migrator
cargo run
```

The API listens on the host/port defined in configuration (defaults to `0.0.0.0:8080`). SQLx migrations live under `backend/migrations` and are normally handled by `./scripts/bootstrap.sh`, but the commands above remain available for manual control.

### Local Frontend Workflow

The combined dev script starts Vite automatically, yet the usual commands still work for focused frontend tasks:

```bash
cd frontend
npm install
npm run dev
```

Visit <http://localhost:3000> (forwarded from Vite’s configured host/port) to access the SPA. The dev server proxies API calls to `VITE_API_BASE` (default `/api`).

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

### Finance Batch History API

Finance roles can retrieve recent NetSuite exports via `GET /api/finance/batches`. The endpoint requires an Authorization
token whose JWT `role` claim resolves to `finance`; all other roles receive HTTP 403. It returns the 25 most recent
`netsuite_batches` ordered by finalization timestamp, and aggregates journal-line counts and amounts for quick history
review.

```json
{
  "batches": [
    {
      "id": "c8d1c55e-7e7b-41aa-8701-d2f3c2ff0e60",
      "batch_reference": "APR-2024-02",
      "finalized_at": "2024-04-30T18:32:15Z",
      "status": "exported",
      "exported_at": "2024-04-30T18:35:18Z",
      "report_count": 6,
      "total_amount_cents": 418500
    }
  ]
}
```

Clients should format `total_amount_cents` according to their preferred currency display (the finance UI assumes USD).
The `status` field mirrors the batch export lifecycle (`pending`, `exported`, etc.) and `exported_at` is `null` until a
batch successfully posts to NetSuite.
