#!/usr/bin/env bash
set -euo pipefail

# WARNING: This script submits an on-chain transaction that costs real CKB.
# Make sure info.txt, blocks.bin, and your private key are all correct before running.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
SDK_DIR="$REPO_ROOT/sdk"
SP1_SCRIPT_DIR="$REPO_ROOT/sp1/ckb-vote-verification/script"

source "$SCRIPT_DIR/env.sh"

# ── Build / type-check SDK ────────────────────────────────────────────────────
echo "==> Building SDK (pnpm dev check)..."
(cd "$SDK_DIR" && pnpm dev check)

INFO_FILE="$SCRIPT_DIR/info.txt"
PK_FILE="$SCRIPT_DIR/pk1"
BLOCKS_BIN="$SDK_DIR/blocks.bin"

# Output files produced by the SP1 prover
PROOF_BIN="$SP1_SCRIPT_DIR/proof-plonk.bin"
PUBLIC_VALUES_BIN="$SP1_SCRIPT_DIR/public-values.bin"

# ── Read info.txt ─────────────────────────────────────────────────────────────
if [[ ! -f "$INFO_FILE" ]]; then
  echo "ERROR: $INFO_FILE not found. Run run-devnet.sh first." >&2
  exit 1
fi

PROPOSAL_TX_HASH=$(grep '^proposal_tx_hash=' "$INFO_FILE" | cut -d= -f2)
PROPOSAL_TX_INDEX=$(grep '^proposal_tx_index=' "$INFO_FILE" | cut -d= -f2)
START_BLOCK_HASH=$(grep '^start_block_hash=' "$INFO_FILE" | cut -d= -f2)
END_BLOCK_HASH=$(grep '^end_block_hash=' "$INFO_FILE" | cut -d= -f2)

if [[ -z "$PROPOSAL_TX_HASH" || -z "$START_BLOCK_HASH" || -z "$END_BLOCK_HASH" ]]; then
  echo "ERROR: info.txt is missing required fields." >&2
  exit 1
fi

echo "==> Loaded from $INFO_FILE:"
echo "  proposal_tx_hash:  $PROPOSAL_TX_HASH"
echo "  proposal_tx_index: $PROPOSAL_TX_INDEX"
echo "  start_block_hash:  $START_BLOCK_HASH"
echo "  end_block_hash:    $END_BLOCK_HASH"

if [[ ! -f "$BLOCKS_BIN" ]]; then
  echo "ERROR: $BLOCKS_BIN not found. Run run-devnet.sh first." >&2
  exit 1
fi

# ── Confirmation prompt ───────────────────────────────────────────────────────
echo ""
echo "WARNING: The next steps will generate a PLONK proof via the SP1 network"
echo "         (which costs prover network credits) "
echo ""
read -r -p "Type 'yes' to continue: " CONFIRM
if [[ "$CONFIRM" != "yes" ]]; then
  echo "Aborted."
  exit 1
fi

# ── Generate PLONK proof via SP1 network ─────────────────────────────────────
echo ""
echo "==> Generating PLONK proof (prove-via-network)..."
(cd "$SP1_SCRIPT_DIR" && cargo run --release -- \
  --prove-via-network \
  --proposal-tx-hash "$PROPOSAL_TX_HASH" \
  --proposal-index "$PROPOSAL_TX_INDEX" \
  --input "$BLOCKS_BIN" \
  --output ".")

if [[ ! -f "$PROOF_BIN" || ! -f "$PUBLIC_VALUES_BIN" ]]; then
  echo "ERROR: expected proof files not found after SP1 run:" >&2
  echo "  $PROOF_BIN" >&2
  echo "  $PUBLIC_VALUES_BIN" >&2
  exit 1
fi
echo "  proof-plonk.bin:    $PROOF_BIN"
echo "  public-values.bin:  $PUBLIC_VALUES_BIN"

# ── Consume the proposal on-chain ─────────────────────────────────────────────
echo ""
echo "==> Consuming proposal on-chain..."
(cd "$SDK_DIR" && pnpm dev consume-proposal \
  --private-key-file "$PK_FILE" \
  --proposal-tx-hash "$PROPOSAL_TX_HASH" \
  --proposal-index "${PROPOSAL_TX_INDEX:-0}" \
  --proof "$PROOF_BIN" \
  --public-values "$PUBLIC_VALUES_BIN" \
  --start-block-hash "$START_BLOCK_HASH" \
  --end-block-hash "$END_BLOCK_HASH")
