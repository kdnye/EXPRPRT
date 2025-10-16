#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$REPO_ROOT"

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

FRONT_PID=""
BACK_PID=""
cleanup() {
  trap - SIGINT SIGTERM EXIT
  if [[ -n "${FRONT_PID}" ]]; then
    kill "${FRONT_PID}" >/dev/null 2>&1 || true
    wait "${FRONT_PID}" 2>/dev/null || true
  fi
  if [[ -n "${BACK_PID}" ]]; then
    kill "${BACK_PID}" >/dev/null 2>&1 || true
    wait "${BACK_PID}" 2>/dev/null || true
  fi
}

trap cleanup SIGINT SIGTERM EXIT

npm run dev -- --host 0.0.0.0 --port 3000 &
FRONT_PID=$!

declare EXIT_CODE=0
(
  cd backend
  cargo run
) &
BACK_PID=$!

while true; do
  if ! kill -0 "${FRONT_PID}" >/dev/null 2>&1; then
    wait "${FRONT_PID}" || EXIT_CODE=$?
    break
  fi
  if ! kill -0 "${BACK_PID}" >/dev/null 2>&1; then
    wait "${BACK_PID}" || EXIT_CODE=$?
    break
  fi
  sleep 1
done

cleanup
exit "${EXIT_CODE}"
