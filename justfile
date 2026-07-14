set dotenv-load := true
set shell := ["zsh", "-cu"]

root := justfile_directory()

default:
    @just --list

# --- Observability (SigNoz via Foundry on Podman) ---

obs-up:
    #!/usr/bin/env zsh
    set -euo pipefail
    source "{{root}}/scripts/podman-env.zsh"
    if ! command -v foundryctl >/dev/null 2>&1; then
      echo "foundryctl not found. Install: curl -fsSL https://signoz.io/foundry.sh | bash"
      exit 1
    fi
    foundryctl cast -f "{{root}}/casting.yaml"
    # Dashboard JSON uses SigNoz Dashboard v2. Foundry regenerates its Compose
    # file, so layer the repository-owned feature override on every startup.
    (cd "{{root}}/pours/deployment" && podman compose -f compose.yaml -f "{{root}}/infra/signoz/compose.dashboard-v2.yaml" up -d --force-recreate signoz-signoz-0)

obs-down:
    #!/usr/bin/env zsh
    set -euo pipefail
    source "{{root}}/scripts/podman-env.zsh"
    if [[ ! -f "{{root}}/pours/deployment/compose.yaml" ]]; then
      echo "No Foundry deployment found under pours/deployment"
      exit 0
    fi
    # Foundry emits Compose files; run them via podman (external docker-compose provider → Podman API).
    (cd "{{root}}/pours/deployment" && podman compose down)

# Sync the checked-in SigNoz dashboard through the authenticated API.
dashboard-sync:
    #!/usr/bin/env zsh
    set -euo pipefail
    source "{{root}}/scripts/podman-env.zsh"
    if [[ -z "${SIGNOZ_API_KEY:-}" ]]; then
      echo "SIGNOZ_API_KEY is unset; create an Editor API key in SigNoz and add it to .env"
      exit 1
    fi
    podman compose -f "{{root}}/infra/compose.yaml" --env-file "{{root}}/.env" --profile dashboard run --rm dashboard-provisioner

# --- App infrastructure (Podman) ---

infra-up:
    #!/usr/bin/env zsh
    set -euo pipefail
    source "{{root}}/scripts/podman-env.zsh"
    if [[ -z "${GARAGE_ADMIN_TOKEN:-}" ]]; then
      echo "GARAGE_ADMIN_TOKEN is unset; run just setup first"
      exit 1
    fi
    if ! podman network exists signoz-network; then
      echo "SigNoz network is missing; run just obs-up first"
      exit 1
    fi
    mkdir -p "{{root}}/infra/logs/caddy" "{{root}}/infra/logs/postgres"
    chmod -R a+rwx "{{root}}/infra/logs/caddy" "{{root}}/infra/logs/postgres" || true
    podman compose -f "{{root}}/infra/compose.yaml" --env-file "{{root}}/.env" up -d postgres garage otel-collector

infra-up-all:
    #!/usr/bin/env zsh
    set -euo pipefail
    source "{{root}}/scripts/podman-env.zsh"
    if [[ -z "${GARAGE_ADMIN_TOKEN:-}" ]]; then
      echo "GARAGE_ADMIN_TOKEN is unset; run just setup first"
      exit 1
    fi
    if ! podman network exists signoz-network; then
      echo "SigNoz network is missing; run just obs-up first"
      exit 1
    fi
    mkdir -p "{{root}}/infra/logs/caddy" "{{root}}/infra/logs/postgres"
    chmod -R a+rwx "{{root}}/infra/logs/caddy" "{{root}}/infra/logs/postgres" || true
    podman compose -f "{{root}}/infra/compose.yaml" --env-file "{{root}}/.env" up -d --build

infra-down:
    #!/usr/bin/env zsh
    set -euo pipefail
    source "{{root}}/scripts/podman-env.zsh"
    podman compose -f "{{root}}/infra/compose.yaml" down

# Stop everything for this POC (app Compose + SigNoz). Use when leaving infra / obs running.
down:
    #!/usr/bin/env zsh
    set -euo pipefail
    source "{{root}}/scripts/podman-env.zsh"
    echo "Stopping app Compose (postgres/garage/otel/backend/frontend/caddy)..."
    podman compose -f "{{root}}/infra/compose.yaml" --env-file "{{root}}/.env" down || true
    if [[ -f "{{root}}/pours/deployment/compose.yaml" ]]; then
      echo "Stopping SigNoz (Foundry)..."
      (cd "{{root}}/pours/deployment" && podman compose down) || true
    else
      echo "No Foundry deployment under pours/deployment (skip SigNoz)"
    fi
    echo "Done. Volumes are kept (data persists). Wipe with: just down-wipe"

# Stop everything and delete Compose volumes (DB / Garage / SigNoz data).
down-wipe:
    #!/usr/bin/env zsh
    set -euo pipefail
    source "{{root}}/scripts/podman-env.zsh"
    echo "Stopping app Compose and removing volumes..."
    podman compose -f "{{root}}/infra/compose.yaml" --env-file "{{root}}/.env" down -v || true
    if [[ -f "{{root}}/pours/deployment/compose.yaml" ]]; then
      echo "Stopping SigNoz and removing volumes..."
      (cd "{{root}}/pours/deployment" && podman compose down -v) || true
    else
      echo "No Foundry deployment under pours/deployment (skip SigNoz)"
    fi
    echo "Done. App + SigNoz containers and volumes removed."
    echo "Next time: just setup && just migrate && just garage-init && just seed"

infra-logs *service:
    #!/usr/bin/env zsh
    set -euo pipefail
    source "{{root}}/scripts/podman-env.zsh"
    podman compose -f "{{root}}/infra/compose.yaml" logs -f {{service}}

# --- Garage bootstrap ---

garage-init:
    #!/usr/bin/env zsh
    set -euo pipefail
    source "{{root}}/scripts/podman-env.zsh"
    echo "Waiting for Garage..."
    sleep 2

    KEY_NAME=equipment-app-key
    BUCKET="${GARAGE_BUCKET:-equipment-images}"

    # Prefer compose service name; fall back to first matching container ID.
    # Do not use podman Go-template --format here — just treats braces as interpolation.
    GARAGE_CID=""
    for name in signozpoc-garage-1 signozpoc_garage_1; do
      if podman inspect "$name" >/dev/null 2>&1; then
        GARAGE_CID=$name
        break
      fi
    done
    if [[ -z "$GARAGE_CID" ]]; then
      GARAGE_CID=$(podman ps -q --filter name=garage | head -1)
    fi
    if [[ -z "$GARAGE_CID" ]]; then
      echo "Garage container not found. Run: just infra-up"
      exit 1
    fi

    g() { podman exec "$GARAGE_CID" /garage "$@"; }

    # dxflrs/garage image exposes the CLI only as /garage (no PATH entry).
    NODE=$(g status 2>/dev/null | awk '/^[0-9a-f]/ {print $1; exit}')
    if [[ -z "$NODE" ]]; then
      echo "Could not read Garage node id from: /garage status"
      g status || true
      exit 1
    fi

    echo "Container: $GARAGE_CID"
    echo "Node: $NODE"
    if g status 2>/dev/null | grep -q 'NO ROLE ASSIGNED'; then
      echo "Assigning layout role..."
      g layout assign -z dc1 -c 1GB "$NODE" >/dev/null
      g layout apply --version 1 >/dev/null
    else
      echo "Cluster layout already applied"
    fi

    # Key IDs whose name column is exactly KEY_NAME (idempotent; dedupe by ID).
    typeset -a KEY_IDS
    KEY_IDS=(${(f)"$(
      g key list 2>/dev/null | awk -v n="$KEY_NAME" '$1 ~ /^GK/ && $3 == n { print $1 }'
    )"})
    if (( ${#KEY_IDS[@]} == 0 )); then
      echo "Creating access key: $KEY_NAME"
      g key create "$KEY_NAME" >/dev/null
      KEY_IDS=(${(f)"$(
        g key list 2>/dev/null | awk -v n="$KEY_NAME" '$1 ~ /^GK/ && $3 == n { print $1 }'
      )"})
    elif (( ${#KEY_IDS[@]} > 1 )); then
      keep="${KEY_IDS[1]}"
      echo "Found ${#KEY_IDS[@]} keys named $KEY_NAME; keeping $keep, deleting extras"
      for ((i = 2; i <= ${#KEY_IDS[@]}; i++)); do
        echo "  delete ${KEY_IDS[i]}"
        g key delete --yes "${KEY_IDS[i]}" >/dev/null || true
      done
      KEY_IDS=("$keep")
    else
      echo "Reusing existing access key: $KEY_NAME (${KEY_IDS[1]})"
    fi
    if (( ${#KEY_IDS[@]} == 0 )); then
      echo "Failed to resolve Garage access key ID for $KEY_NAME"
      g key list || true
      exit 1
    fi
    KEY_ID="${KEY_IDS[1]}"

    if g bucket list 2>/dev/null | awk -v b="$BUCKET" 'NR > 1 && index($0, b) { found = 1 } END { exit !found }'; then
      echo "Bucket already exists: $BUCKET"
    else
      echo "Creating bucket: $BUCKET"
      g bucket create "$BUCKET" >/dev/null
    fi
    g bucket allow --read --write --owner "$BUCKET" --key "$KEY_ID" >/dev/null || true

    SECRET=$(g key info --show-secret "$KEY_ID" 2>/dev/null | awk '/Secret key:/ { print $NF; exit }')
    echo "Created/ensured bucket $BUCKET"
    echo "Key ID: $KEY_ID"
    if [[ -n "${SECRET:-}" ]]; then
      echo "Secret key: $SECRET"
      echo "Set these in .env as GARAGE_ACCESS_KEY / GARAGE_SECRET_KEY if they differ."
    else
      echo "Could not print secret (re-run: podman exec $GARAGE_CID /garage key info --show-secret $KEY_ID)"
    fi

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
    cd "{{root}}/backend" && cargo run --bin equipment_reservation

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
    echo "Optional gateway: just infra-up-all  (includes caddy)"

setup:
    #!/usr/bin/env zsh
    set -euo pipefail
    if [[ ! -f "{{root}}/.env" ]]; then
      cp "{{root}}/.env.example" "{{root}}/.env"
      echo "Created .env from .env.example"
    fi
    if ! grep -Eq '^GARAGE_ADMIN_TOKEN=.+$' "{{root}}/.env"; then
      token="$(openssl rand -base64 32 | tr -d '\n')"
      if grep -q '^GARAGE_ADMIN_TOKEN=' "{{root}}/.env"; then
        GARAGE_SETUP_TOKEN="$token" perl -0pi -e 's/^GARAGE_ADMIN_TOKEN=.*$/GARAGE_ADMIN_TOKEN=$ENV{GARAGE_SETUP_TOKEN}/m' "{{root}}/.env"
      else
        printf '\nGARAGE_ADMIN_TOKEN=%s\n' "$token" >> "{{root}}/.env"
      fi
      echo "Generated a local Garage admin token"
    fi
    legacy_proxies='10.0.0.0/8,172.16.0.0/12,192.168.0.0/16,127.0.0.1/32,::1/128'
    if grep -Fqx "TRUSTED_PROXIES=$legacy_proxies" "{{root}}/.env"; then
      perl -0pi -e 's/^TRUSTED_PROXIES=.*$/TRUSTED_PROXIES=172.30.0.0\/24/m' "{{root}}/.env"
      echo "Narrowed TRUSTED_PROXIES to the dedicated Caddy proxy network"
    fi
    just frontend-install
    just obs-up
    just infra-up
    echo "Run migrations after Postgres is healthy: just migrate"
    echo "Bootstrap Garage: just garage-init"
    echo "SigNoz was started before the gateway Collector so it can join the SigNoz network"
