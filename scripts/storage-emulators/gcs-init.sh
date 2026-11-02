#!/usr/bin/env bash
# Ensure the shared fake-gcs-server bucket exists (retries until the HTTP API is up).
set -euo pipefail

BUCKET="${STORAGE_E2E_BUCKET:-doido-test}"
ENDPOINT="${STORAGE_E2E_GCS_ENDPOINT:-http://127.0.0.1:4443}"
deadline=$((SECONDS + 60))

while (( SECONDS < deadline )); do
  if curl -sf "${ENDPOINT}/storage/v1/b/${BUCKET}" >/dev/null 2>&1; then
    echo "==> GCS bucket ${BUCKET} ready at ${ENDPOINT}"
    exit 0
  fi
  if curl -sf -X POST "${ENDPOINT}/storage/v1/b?project=doido-test" \
    -H "Content-Type: application/json" \
    -d "{\"name\":\"${BUCKET}\"}" >/dev/null 2>&1; then
    echo "==> GCS bucket ${BUCKET} created at ${ENDPOINT}"
    exit 0
  fi
  sleep 1
done

echo "error: GCS bucket ${BUCKET} not ready at ${ENDPOINT} after 60s" >&2
exit 1
