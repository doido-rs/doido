#!/usr/bin/env bash
# Floci/LocalStack-compat ready hook — create the shared e2e bucket.
set -euo pipefail
aws s3 mb "s3://${STORAGE_E2E_BUCKET:-doido-test}" || true
