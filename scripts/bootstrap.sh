#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$REPO_ROOT"

# Ensure a working environment file is present for local runs.
if [[ ! -f "${REPO_ROOT}/.env" ]]; then
  if [[ -f "${REPO_ROOT}/.env.example" ]]; then
    echo "Creating .env from .env.example..."
    cp "${REPO_ROOT}/.env.example" "${REPO_ROOT}/.env"
  else
    cat <<'EOF' >&2
No .env found and .env.example is missing.
Restore the sample configuration (git restore --source=origin/main -- .env.example)
or create a local .env before rerunning bootstrap.
EOF
    exit 1
  fi
fi

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

HOST_PORT="${POSTGRES_HOST_PORT:-5432}"

if [[ -n "${HOST_PORT}" ]] && ! [[ "${HOST_PORT}" =~ ^[0-9]+$ ]]; then
  cat <<EOF >&2
POSTGRES_HOST_PORT must be a numeric port value. Received '${HOST_PORT}'.
Update your environment or .env and rerun this script.
EOF
  exit 1
fi

if [[ -n "${HOST_PORT}" ]] && command -v python3 >/dev/null 2>&1; then
  if ! python3 - "$HOST_PORT" <<'PY'
import socket
import sys

port = int(sys.argv[1])

with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as sock:
    try:
        sock.bind(("0.0.0.0", port))
    except OSError:
        sys.exit(1)
sys.exit(0)
PY
  then
    cat <<EOF >&2
Port ${HOST_PORT} is already in use on this machine. Docker Compose cannot start PostgreSQL until the port is free.
Set POSTGRES_HOST_PORT to an unused port (for example 55432) in .env or in the environment before rerunning this script:
  POSTGRES_HOST_PORT=55432 ./scripts/bootstrap.sh
EOF
    exit 1
  fi
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

if ! command -v cc >/dev/null 2>&1; then
  cat <<'EOF' >&2
No C compiler detected on PATH. The Rust toolchain needs a system linker such as gcc.
Install build essentials (for example, on Debian/Ubuntu: sudo apt-get install build-essential pkg-config libssl-dev)
and rerun this script.
EOF
  exit 1
fi

echo "Running database migrations..."
cargo run --manifest-path backend/Cargo.toml --bin migrator
