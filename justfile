set dotenv-load := true
set shell := ["zsh", "-cu"]

root := justfile_directory()

default:
    @just --list

# --- Observability (SigNoz via Foundry) ---

obs-up:
    #!/usr/bin/env zsh
    set -euo pipefail
    if ! command -v foundryctl >/dev/null 2>&1; then
      echo "foundryctl not found. Install: curl -fsSL https://signoz.io/foundry.sh | bash"
      exit 1
    fi
    if [[ -n "${DOCKER_HOST:-}" ]]; then
      echo "Using DOCKER_HOST=$DOCKER_HOST"
    elif command -v podman >/dev/null 2>&1; then
      echo "Tip: for Podman, export DOCKER_HOST to the Podman API socket before casting."
    fi
    foundryctl cast -f "{{root}}/casting.yaml"

obs-down:
    #!/usr/bin/env zsh
    set -euo pipefail
    if [[ -f "{{root}}/pours/deployment/compose.yaml" ]]; then
      (cd "{{root}}/pours/deployment" && docker compose down)
    else
      echo "No Foundry deployment found under pours/deployment"
    fi

# --- App infrastructure (Podman Compose) ---

infra-up:
    mkdir -p "{{root}}/infra/logs/caddy" "{{root}}/infra/logs/postgres"
    chmod -R a+rwx "{{root}}/infra/logs/caddy" "{{root}}/infra/logs/postgres" || true
    podman compose -f "{{root}}/infra/compose.yaml" --env-file "{{root}}/.env" up -d postgres garage otel-collector

infra-up-all:
    mkdir -p "{{root}}/infra/logs/caddy" "{{root}}/infra/logs/postgres"
    chmod -R a+rwx "{{root}}/infra/logs/caddy" "{{root}}/infra/logs/postgres" || true
    podman compose -f "{{root}}/infra/compose.yaml" --env-file "{{root}}/.env" up -d --build

infra-down:
    podman compose -f "{{root}}/infra/compose.yaml" down

infra-logs *service:
    podman compose -f "{{root}}/infra/compose.yaml" logs -f {{service}}

# --- Garage bootstrap ---

garage-init:
    #!/usr/bin/env zsh
    set -euo pipefail
    echo "Waiting for Garage..."
    sleep 2
    NODE=$(podman exec signozpoc_garage_1 garage status 2>/dev/null | awk '/^([0-9a-f]{16})/ {print $1; exit}' || true)
    if [[ -z "${NODE}" ]]; then
      # Compose project naming may differ; try common names
      CID=$(podman ps --filter name=garage --format '{{.ID}}' | head -1)
      NODE=$(podman exec "$CID" garage status | awk '/^[0-9a-f]/ {print $1; exit}')
      GARAGE_CID=$CID
    else
      GARAGE_CID=signozpoc_garage_1
    fi
    echo "Node: $NODE"
    podman exec "$GARAGE_CID" garage layout assign -z dc1 -c 1G "$NODE" || true
    podman exec "$GARAGE_CID" garage layout apply --version 1 || true
    podman exec "$GARAGE_CID" garage key create equipment-app-key || true
    podman exec "$GARAGE_CID" garage bucket create "${GARAGE_BUCKET:-equipment-images}" || true
    KEY_ID=$(podman exec "$GARAGE_CID" garage key info equipment-app-key | awk '/Key ID/ {print $NF; exit}')
    SECRET=$(podman exec "$GARAGE_CID" garage key info equipment-app-key | awk '/Secret key/ {print $NF; exit}')
    podman exec "$GARAGE_CID" garage bucket allow --read --write --owner "${GARAGE_BUCKET:-equipment-images}" --key equipment-app-key || true
    echo "Created/ensured bucket ${GARAGE_BUCKET:-equipment-images}"
    echo "Key ID: $KEY_ID"
    echo "Update .env GARAGE_ACCESS_KEY / GARAGE_SECRET_KEY if needed."

# --- Backend ---

migrate:
    #!/usr/bin/env zsh
    set -euo pipefail
    cd "{{root}}"
    set -a; source .env; set +a
    sqlx migrate run --source backend/migrations

seed:
    #!/usr/bin/env zsh
    set -euo pipefail
    cd "{{root}}"
    set -a; source .env; set +a
    cargo run --manifest-path backend/Cargo.toml --bin seed

backend-test:
    cd "{{root}}/backend" && cargo test

backend-dev:
    cd "{{root}}/backend" && cargo run

# --- Frontend ---

frontend-install:
    cd "{{root}}/frontend" && vp install

frontend-test:
    cd "{{root}}/frontend" && vp test --run

frontend-dev:
    cd "{{root}}/frontend" && vp dev --host 0.0.0.0 --port 5173

frontend-check:
    cd "{{root}}/frontend" && vp check

# --- Combined ---

test: backend-test frontend-test

dev:
    #!/usr/bin/env zsh
    set -euo pipefail
    just infra-up
    echo "Start backend: just backend-dev"
    echo "Start frontend: just frontend-dev"
    echo "Optional gateway: podman compose -f infra/compose.yaml up -d caddy"

setup:
    #!/usr/bin/env zsh
    set -euo pipefail
    if [[ ! -f "{{root}}/.env" ]]; then
      cp "{{root}}/.env.example" "{{root}}/.env"
      echo "Created .env from .env.example"
    fi
    just frontend-install
    just infra-up
    echo "Run migrations after Postgres is healthy: just migrate"
    echo "Bootstrap Garage: just garage-init"
    echo "Start SigNoz: just obs-up"
