#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$REPO_ROOT"

npm install
npm install --prefix frontend

if ! command -v docker >/dev/null 2>&1; then
  echo "Docker is not available; skipping PostgreSQL bootstrap." >&2
  exit 0
fi

if docker compose version >/dev/null 2>&1; then
  COMPOSE_CMD=(docker compose)
elif command -v docker-compose >/dev/null 2>&1; then
  COMPOSE_CMD=(docker-compose)
else
  echo "Docker Compose is not available; skipping PostgreSQL bootstrap." >&2
  exit 0
fi

echo "Starting PostgreSQL via docker compose..."
"${COMPOSE_CMD[@]}" up -d db >/dev/null

# Wait for the container ID to become available.
DB_CONTAINER=""
for _ in {1..30}; do
  DB_CONTAINER="$({ "${COMPOSE_CMD[@]}" ps -q db; } 2>/dev/null || true)"
  if [[ -n "${DB_CONTAINER}" ]]; then
    break
  fi
  sleep 1
done

if [[ -z "${DB_CONTAINER}" ]]; then
  echo "Failed to locate the PostgreSQL container (service 'db')." >&2
  exit 1
fi

echo "Waiting for PostgreSQL to become ready..."
for _ in {1..60}; do
  if docker exec "${DB_CONTAINER}" pg_isready -U expenses -d expenses >/dev/null 2>&1; then
    DB_READY=1
    break
  fi
  sleep 1
done

if [[ "${DB_READY:-0}" -ne 1 ]]; then
  echo "PostgreSQL did not become ready in time." >&2
  exit 1
fi

# Load environment variables for the migrator. Prefer developer overrides.
ENV_FILE="${REPO_ROOT}/.env"
if [[ -f "${ENV_FILE}" ]]; then
  set -a
  # shellcheck source=/dev/null
  source "${ENV_FILE}"
  set +a
elif [[ -f "${REPO_ROOT}/.env.example" ]]; then
  set -a
  # shellcheck source=/dev/null
  source "${REPO_ROOT}/.env.example"
  set +a
fi

if ! command -v cargo >/dev/null 2>&1; then
  cat <<'EOF' >&2
Rust toolchain not detected. Install Rust via rustup before running the migrator:

  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

Then restart your shell so that cargo is on the PATH and rerun this script.
EOF
  exit 1
fi

echo "Running database migrations..."
cargo run --manifest-path backend/Cargo.toml --bin migrator
