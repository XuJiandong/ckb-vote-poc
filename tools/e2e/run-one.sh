#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SDK_DIR="$REPO_ROOT/sdk"
SP1_SCRIPT_DIR="$REPO_ROOT/sp1/ckb-vote-verification/script"

DURATION=3
DESCRIPTION="${1:-test1}"
PK_FILE="$SCRIPT_DIR/pk1"
BLOCKS_BIN="$SDK_DIR/blocks.bin"

echo "==> Creating proposal (duration=$DURATION, description=\"$DESCRIPTION\")..."
create_output=$(cd "$SDK_DIR" && pnpm dev create-proposal \
  --private-key-file "$PK_FILE" \
  --duration "$DURATION" \
  --description "$DESCRIPTION")
echo "$create_output"

# Parse tx hash from "  outpoint:     0x<txhash>:0"
PROPOSAL_TX_HASH=$(echo "$create_output" | grep 'outpoint:' | sed 's/.*outpoint:[[:space:]]*//' | cut -d: -f1)
if [[ -z "$PROPOSAL_TX_HASH" ]]; then
  echo "ERROR: failed to parse proposal tx hash from output" >&2
  exit 1
fi
echo ""
echo "  proposal tx hash: $PROPOSAL_TX_HASH"

# Wait for the next block before voting (~10s per block, 15s to be safe)
echo ""
echo "==> Waiting 15 seconds before voting (to land in a different block)..."
sleep 15

echo ""
echo "==> Voting yes on proposal $PROPOSAL_TX_HASH..."
(cd "$SDK_DIR" && pnpm dev vote \
  --private-key-file "$PK_FILE" \
  --proposal-tx-hash "$PROPOSAL_TX_HASH" \
  --vote yes)

# Need ≥4 blocks total from proposal creation (~40s); 25s more after the 15s already elapsed
echo ""
echo "==> Waiting 25 more seconds for the proposal period to complete (≥4 blocks total)..."
sleep 25

echo ""
echo "==> Dumping $DURATION blocks to $BLOCKS_BIN..."
(cd "$SDK_DIR" && pnpm dev dump-blocks \
  --proposal-tx-hash "$PROPOSAL_TX_HASH" \
  --count "$DURATION" \
  --out "$BLOCKS_BIN")

echo ""
echo "==> Running SP1 verification..."
(cd "$SP1_SCRIPT_DIR" && cargo run --release -- \
  --execute \
  --proposal-tx-hash "$PROPOSAL_TX_HASH" \
  --proposal-index 0 \
  --input "$BLOCKS_BIN")
