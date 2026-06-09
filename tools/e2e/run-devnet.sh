#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SDK_DIR="$REPO_ROOT/sdk"
SP1_SCRIPT_DIR="$REPO_ROOT/sp1/ckb-vote-verification/script"

source "$SCRIPT_DIR/env.sh"

DURATION=3
DESCRIPTION="${1:-test1}"
PK_FILE="$SCRIPT_DIR/pk1"
BLOCKS_BIN="$SDK_DIR/blocks.bin"

# Poll until a TX is committed on-chain; prints its decimal block number to stdout.
poll_tx_committed() {
  local tx_hash="$1"
  local deadline=$(( SECONDS + 60 ))
  while [ "$SECONDS" -lt "$deadline" ]; do
    local resp
    resp=$(curl -s "$CKB_RPC" -X POST -H 'Content-Type: application/json' \
      -d "{\"jsonrpc\":\"2.0\",\"method\":\"get_transaction\",\"params\":[\"$tx_hash\"],\"id\":1}")
    local status
    status=$(echo "$resp" | python3 -c \
      "import json,sys; r=(json.load(sys.stdin).get('result') or {}); print(r.get('tx_status',{}).get('status','none'))" \
      2>/dev/null || echo "none")
    if [ "$status" = "committed" ]; then
      echo "$resp" | python3 -c \
        "import json,sys; print(int(json.load(sys.stdin)['result']['tx_status']['block_number'],16))" \
        2>/dev/null
      return 0
    fi
    sleep 0.5
  done
  echo "ERROR: TX $tx_hash not committed within 60 s" >&2
  return 1
}

# Poll until the chain tip is at or beyond target_block.
wait_for_block() {
  local target="$1"
  local deadline=$(( SECONDS + 120 ))
  while [ "$SECONDS" -lt "$deadline" ]; do
    local tip
    tip=$(curl -s "$CKB_RPC" -X POST -H 'Content-Type: application/json' \
      -d '{"jsonrpc":"2.0","method":"get_tip_block_number","params":[],"id":1}' \
      | python3 -c "import json,sys; print(int(json.load(sys.stdin)['result'],16))" \
      2>/dev/null || echo "0")
    if [ "$tip" -ge "$target" ]; then
      return 0
    fi
    sleep 0.5
  done
  echo "ERROR: tip did not reach block $target within 120 s" >&2
  return 1
}

echo "==> Creating proposal (duration=$DURATION, description=\"$DESCRIPTION\")..."
create_output=$(cd "$SDK_DIR" && pnpm dev create-proposal \
  --private-key-file "$PK_FILE" \
  --duration "$DURATION" \
  --description "$DESCRIPTION")
echo "$create_output"

PROPOSAL_TX_HASH=$(echo "$create_output" | grep 'outpoint:' | sed 's/.*outpoint:[[:space:]]*//' | cut -d: -f1)
if [[ -z "$PROPOSAL_TX_HASH" ]]; then
  echo "ERROR: failed to parse proposal tx hash from output" >&2
  exit 1
fi
echo ""
echo "  proposal tx hash: $PROPOSAL_TX_HASH"

echo ""
echo "==> Waiting for proposal TX to be committed..."
PROPOSAL_BLOCK=$(poll_tx_committed "$PROPOSAL_TX_HASH")
echo "  proposal committed in block $PROPOSAL_BLOCK"
END_BLOCK=$(( PROPOSAL_BLOCK + DURATION ))

echo ""
echo "==> Voting yes on proposal $PROPOSAL_TX_HASH..."
(cd "$SDK_DIR" && pnpm dev vote \
  --private-key-file "$PK_FILE" \
  --proposal-tx-hash "$PROPOSAL_TX_HASH" \
  --vote yes)

echo ""
echo "==> Waiting for voting window to close (need tip ≥ block $END_BLOCK)..."
wait_for_block "$END_BLOCK"
echo "  tip reached block $END_BLOCK"

echo ""
echo "==> Dumping $((DURATION + 1)) blocks to $BLOCKS_BIN..."
(cd "$SDK_DIR" && pnpm dev dump-blocks \
  --proposal-tx-hash "$PROPOSAL_TX_HASH" \
  --count "$((DURATION + 1))" \
  --out "$BLOCKS_BIN")

echo ""
echo "==> Running SP1 verification..."
(cd "$SP1_SCRIPT_DIR" && cargo run --release -- \
  --execute \
  --proposal-tx-hash "$PROPOSAL_TX_HASH" \
  --proposal-index 0 \
  --input "$BLOCKS_BIN")
