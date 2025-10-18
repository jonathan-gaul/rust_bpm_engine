Devcontainer for rustbpm

Usage

1. Open the repo in VS Code. When prompted, reopen in container (or use the Remote - Containers: Reopen in Container command).
2. The container will build from `.devcontainer/Dockerfile` and start a `postgres` service via docker-compose.
3. The container environment variable `DATABASE_URL` is set to: `postgres://dev:example@postgres:5432/rustbpm`.
4. The Postgres init SQL in `.devcontainer/pg-init/001_schema.sql` creates the required tables on container start.

Commands

Inside the container you can run:

```bash
# build
cargo build

# run tests
cargo test
```

Notes

- The devcontainer bundles the Rust toolchain and system deps needed for building native crates and talking to Postgres.
- If you change DB schema, update the init SQL or add proper migration tooling (e.g. sqlx-cli or refinery).