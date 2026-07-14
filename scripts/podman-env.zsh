# Sourced by just recipes. Forces Docker-compatible CLIs onto the Podman machine API
# (never Colima / Docker Desktop), even if DOCKER_HOST is already set in the shell.

if ! command -v podman >/dev/null 2>&1; then
  echo "podman is required for this project."
  exit 1
fi

if ! podman info >/dev/null 2>&1; then
  echo "Podman is not reachable. Start the machine:"
  echo "  podman machine start"
  exit 1
fi

_sock="${HOME}/.local/share/containers/podman/machine/podman.sock"
if [[ ! -S "$_sock" ]]; then
  _sock="$(podman machine inspect --format '{{.ConnectionInfo.PodmanSocket.Path}}' 2>/dev/null | head -1)"
fi
if [[ -z "${_sock:-}" || ! -S "$_sock" ]]; then
  echo "Podman API socket not found. Is the machine running?"
  echo "  podman machine start"
  exit 1
fi

export DOCKER_HOST="unix://${_sock}"
export CONTAINER_HOST="$DOCKER_HOST"
# Ignore a dead Colima/default docker context when DOCKER_HOST is set.
export DOCKER_CONTEXT=default
unset _sock

echo "Using Podman DOCKER_HOST=$DOCKER_HOST"
