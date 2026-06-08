#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEVNET_DIR="$(cd "$SCRIPT_DIR/../../devnet" && pwd)"

echo "ckb:     $(which ckb)"
echo "ckb-cli: $(which ckb-cli)"
ckb --version
ckb-cli --version

ckb miner -C "$DEVNET_DIR" >/dev/null 2>&1 &

exec ckb run -C "$DEVNET_DIR"
