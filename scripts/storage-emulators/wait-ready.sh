#!/usr/bin/env bash
# Wait until storage emulators started by `make services-up` accept connections.
set -euo pipefail

wait_tcp() {
  local host=$1 port=$2 label=$3
  local deadline=$((SECONDS + 90))
  while (( SECONDS < deadline )); do
    if (echo >/dev/tcp/"$host"/"$port") >/dev/null 2>&1; then
      echo "==> $label ready on $host:$port"
      return 0
    fi
    sleep 1
  done
  echo "error: $label not ready on $host:$port after 90s" >&2
  return 1
}

wait_tcp 127.0.0.1 4566 "Floci (S3)"
wait_tcp 127.0.0.1 10000 "Azurite (blob)"
wait_tcp 127.0.0.1 4443 "fake-gcs-server"
