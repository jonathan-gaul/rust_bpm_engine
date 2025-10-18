#!/bin/sh
# Helper to connect to the project's Postgres using sensible defaults.
# Usage:
#   ./psql.sh            # uses defaults from devcontainer (dev/example, host depends on environment)
#   HOST=localhost ./psql.sh
#   USER=dev PASSWORD=example DB=rustbpm ./psql.sh

# Defaults (mirror .devcontainer/devcontainer.json DATABASE_URL)
USER=${USER:-dev}
PASSWORD=${PASSWORD:-example}
DB=${DB:-rustbpm}

# If running inside a container (detect /.dockerenv), use compose service name 'postgres'
# otherwise, assume host is localhost (container port is forwarded)
if [ -f /.dockerenv ]; then
  HOST=${HOST:-postgres}
else
  HOST=${HOST:-localhost}
fi

export PGPASSWORD=${PASSWORD}
exec psql -h "$HOST" -U "$USER" -d "$DB" "$@"
