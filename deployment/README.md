# Deployment

This guide focuses on standing up the backend API in production-like environments.

## Database migrations

Migrations are **not** executed automatically when the API boots. Apply them as a
separate step before starting the service. Three options are supported:

1. Run the dedicated migrator binary from the backend crate:

   ```bash
   cd backend
   cargo run --bin migrator
   ```

   The command reads the same environment variables as the API and connects using
   `EXPENSES__DATABASE__URL`.

2. Use the SQLx CLI (requires `cargo install sqlx-cli`):

   ```bash
   cd backend
   cargo sqlx migrate run
   ```

3. Run the Dockerized migrator service (requires Docker Compose):

   ```bash
   docker compose run --rm migrator
   ```

   The service waits for the `db` container to become healthy, executes the
   compiled `migrator` binary from the backend image, and exits. Subsequent
   `docker compose up backend` invocations will automatically trigger the
   migrator before launching the API container.

Only after the migrations complete successfully should the API be launched
(e.g. `cargo run`, the Docker image entrypoint, or your process supervisor).
This ensures schema changes are fully applied before any requests are served.
