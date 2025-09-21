#!/usr/bin/env bash
set -euo pipefail

# Resolve repo root and load .env if present
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

if [[ -f "$REPO_ROOT/.env" ]]; then
  set -a
  # shellcheck source=/dev/null
  source "$REPO_ROOT/.env"
  set +a
fi

# Configuration (override via env)
LATENCY_REQUESTS=${LATENCY_REQUESTS:-10}
LATENCY_CONCURRENCY=${LATENCY_CONCURRENCY:-10}
THROUGHPUT_CONCURRENCY=${THROUGHPUT_CONCURRENCY:-10}

# Output directory (default: repo_root/results)
RESULTS_DIR=${RESULTS_DIR:-"$REPO_ROOT/results"}
mkdir -p "$RESULTS_DIR"

# Timestamp for filenames
STAMP=$(date +"%Y-%m-%d_%H-%M-%S")

# Validate environment
if [[ -z "${SUDO_API_KEY:-}" ]]; then
  echo "ERROR: SUDO_API_KEY is not set. Export it before running." >&2
  exit 1
fi

# Default API base URL if not provided
export SUDO_API_BASE_URL=${SUDO_API_BASE_URL:-https://sudoapp.dev/api}

# Models list (CSV)
MODELS_CSV=${MODELS_CSV:-"gemini-2.5-pro"}

echo "== Building bench binary =="
(cd "$REPO_ROOT" && cargo build --release)
BIN="$REPO_ROOT/target/release/bench"

echo "== Running LATENCY (streaming default) for models: $MODELS_CSV =="
LATENCY_LOG="$RESULTS_DIR/latency-$STAMP.log"
"$BIN" latency \
  --requests "$LATENCY_REQUESTS" \
  --concurrency "$LATENCY_CONCURRENCY" \
  --model "$MODELS_CSV" \
  | tee "$LATENCY_LOG"

echo "== Running THROUGHPUT (always streaming) for models: $MODELS_CSV =="
THROUGHPUT_LOG="$RESULTS_DIR/throughput-$STAMP.log"
"$BIN" throughput \
  --concurrency "$THROUGHPUT_CONCURRENCY" \
  --model "$MODELS_CSV" \
  | tee "$THROUGHPUT_LOG"

printf "\nResults written to:\n"
echo "  $LATENCY_LOG"
echo "  $THROUGHPUT_LOG"

