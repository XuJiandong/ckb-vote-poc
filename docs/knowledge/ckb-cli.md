# ckb-cli Knowledge

This document summarizes all knowledge from the [ckb-cli GitHub Wiki](https://github.com/nervosnetwork/ckb-cli/wiki).

## Overview

`ckb-cli` is the official command-line tool for interacting with the CKB (Nervos Common Knowledge Base) blockchain. It supports account management, wallet operations, RPC calls, NervosDAO interactions, contract deployment, simple UDT (SUDT) token operations, and complex transaction construction.

---

## Installation & Getting Started

### Install

Download from [ckb releases](https://github.com/nervosnetwork/ckb/releases) (bundled with ckb) or from the [ckb-cli release page](https://github.com/nervosnetwork/ckb-cli/releases). Build from source:

```sh
git clone https://github.com/nervosnetwork/ckb-cli.git
cd ckb-cli
cargo install -f --path .
```

### Create an Account

Uses secp256k1 signature algorithm and Web3 keystore format. Supports HD wallet (BIP32).

```sh
ckb-cli account new
```

Output: mainnet address, testnet address, `lock_arg`, `lock_hash`. Keystore is saved as `~/.ckb-cli/keystore/UTC--<timestamp>--<lock_arg>`.

### Interactive Mode

Enter `ckb-cli` without arguments to start interactive mode. The CLI automatically syncs a local index with the CKB node. Query sync status with the `info` command inside interactive mode.

---

## Sub Commands Reference

### `rpc` — RPC Calls to Node

| Sub-command | Description |
|---|---|
| `get_block` | Get block by hash |
| `get_block_by_number` | Get block by block number |
| `get_block_hash` | Get block hash by block number |
| `get_header` | Get block header by hash |
| `get_header_by_number` | Get block header by block number |
| `get_tip_header` | Get tip header |
| `get_tip_block_number` | Get tip block number |
| `get_transaction` | Get transaction by hash |
| `get_live_cell` | Get live (unspent) cell |
| `get_current_epoch` | Get current epoch information |
| `get_epoch_by_number` | Get epoch information by epoch number |
| `get_cellbase_output_capacity_details` | Get cellbase output capacity details |
| `get_blockchain_info` | Get chain information |
| `tx_pool_info` | Get transaction pool information |
| `get_cells_by_lock_hash` | Get cells by lock script hash |
| `index_lock_hash` | Create index for live cells/transactions by lock hash |
| `deindex_lock_hash` | Remove index for lock hash |
| `get_live_cells_by_lock_hash` | Get live cells by lock hash (requires index) |
| `get_transactions_by_lock_hash` | Get transactions by lock hash (requires index) |
| `get_peers` | Get connected peers |
| `local_node_info` | Get local node info |
| `add_node` | Connect to a node |
| `remove_node` | Disconnect a node |
| `get_banned_addresses` | Get banned IPs/Subnets |
| `set_ban` | Insert/delete ban entries |
| `broadcast_transaction` | Broadcast transaction without verify |

### `wallet` — Transfer / Query Balance / Key Utils

| Sub-command | Description |
|---|---|
| `transfer` | Transfer capacity to an address (supports data) |
| `get-capacity` | Query capacity by lock script hash / address / lock arg / pubkey |
| `get-live-cells` | Get live cells by lock/type/code hash |
| `db-metrics` | Show index database metrics |
| `top-capacity` | Show top n capacity by lock script hash |

### `dao` — NervosDAO Operations

| Sub-command | Description |
|---|---|
| `deposit` | Deposit capacity into NervosDAO |
| `prepare` | Prepare specified cells from NervosDAO for withdrawal (Phase 1) |
| `withdraw` | Withdraw specified cells from NervosDAO (Phase 2) |
| `query-deposited-cells` | Query deposited capacity by lock script hash or address |
| `query-prepared-cells` | Query prepared capacity by lock script hash or address |

See the [Nervos DAO Operations](#nervos-dao-operations) section below for full details.

### `account` — Account Management

| Sub-command | Description |
|---|---|
| `list` | List all accounts |
| `new` | Create a new account |
| `import` | Import unencrypted private key from file |
| `import-keystore` | Import from encrypted keystore JSON file |
| `unlock` | Unlock an account |
| `update` | Update account password |
| `export` | Export master private key and chain code as hex (USE WITH CAUTION) |
| `bip44-addresses` | Extended receiving/change addresses (BIP-44) |
| `extended-address` | Extended address (BIP-44) |

### `util` — Utilities

| Sub-command | Description |
|---|---|
| `key-info` | Show public info of a secp256k1 private/public key |
| `sign-data` | Sign data with secp256k1 |
| `sign-message` | Sign message with secp256k1 |
| `verify-signature` | Verify compact format signature |
| `eaglesong` | Hash binary with Eaglesong algorithm |
| `blake2b` | Hash binary with Blake2b (personalization: `ckb-default-hash`) |
| `compact-to-difficulty` | Convert compact target to difficulty |
| `difficulty-to-compact` | Convert difficulty to compact target |
| `to-genesis-multisig-addr` | Convert single-sig address to multisig format (mainnet genesis cells only) |
| `to-multisig-addr` | Convert single-sig address to multisig format |

### `molecule` — Molecule Encode/Decode

| Sub-command | Description |
|---|---|
| `decode` | Decode molecule type from binary |
| `encode` | Encode molecule type from JSON to binary |
| `default` | Print default JSON structure of a molecule type |

### `tx` — Complex Transaction Handling

| Sub-command | Description |
|---|---|
| `init` | Init a common sighash/multisig transaction (produces `tx.json`) |
| `add-multisig-config` | Add multisig configuration |
| `clear-field` | Remove all field items in transaction |
| `add-input` | Add cell input (with secp/multisig lock) |
| `add-output` | Add cell output |
| `add-signature` | Add signature |
| `info` | Show transaction details (capacity, tx-fee, etc.) |
| `sign-inputs` | Sign all sighash/multisig inputs |
| `send` | Send multisig transaction |
| `build-multisig-address` | Build multisig address with config and optional since argument |

### `mock-tx` — Mock Transactions

| Sub-command | Description |
|---|---|
| `template` | Print mock transaction template |
| `complete` | Complete the mock transaction |
| `verify` | Verify a mock transaction locally |
| `send` | Complete then send a transaction |

---

## Key Concepts

### Transaction File Format (`tx.json`)

The `tx` subcommand operates on a JSON transaction file with three sections:

```json
{
  "transaction": { "version": "0x0", "cell_deps": [], "header_deps": [], "inputs": [], "outputs": [], "outputs_data": [], "witnesses": [] },
  "multisig_configs": {},
  "signatures": {}
}
```

- **`transaction`**: The raw transaction structure (inputs, outputs, deps, witnesses).
- **`multisig_configs`**: Maps from `lock_arg` to multisig configuration (`sighash_addresses`, `require_first_n`, `threshold`).
- **`signatures`**: Collected signatures awaiting final assembly. Supports multi-party signing — sign on different machines, then combine signatures.

### Lock Script Types

- **sighash (secp256k1_blake160_sighash_all)**: Default lock, requires signature from the owner's private key.
- **multisig (secp256k1_blake160_multisig_all)**: Requires M-of-N signatures. Build with `tx build-multisig-address`.
- **anyone-can-pay (ACP)**: Allows anyone to deposit tokens into the cell without needing to unlock it.
- **cheque**: Enables issuing/transferring SUDT to an address without the receiver needing CKB capacity upfront.

### Timelocked Addresses

Addresses can be timelocked using an epoch-based `since` value. Two methods:

1. **Genesis cells**: Use `util to-genesis-multisig-addr --locktime <date>`
2. **Non-genesis cells**: Use `util to-multisig-addr --locktime <ISO8601-date>` or `tx build-multisig-address --since-absolute-epoch <epoch>`

To transfer FROM a timelocked address after it unlocks, use `--from-locked-address` with the timelocked address and `--from-account` with the corresponding normal address.

---

## Nervos DAO Operations

### Overview

Nervos DAO is a built-in smart contract (type script) on CKB that allows users to deposit CKB and receive compensation for secondary issuance dilution. The DAO has a three-phase lifecycle: **Deposit** → **Prepare (Phase 1 withdraw)** → **Withdraw (Phase 2 withdraw)**. The entire process is governed by the on-chain DAO type script, a system contract hard-coded into the genesis block.

### DAO Type Script

The Nervos DAO type script identifies DAO cells on-chain:

- **Code hash**: `0x82d76d1b75fe2fd9a27dfbaa65a039221a380d76c926f378d3f81cf3e7e13f2e`
- **Hash type**: `Type`
- **Args**: Empty (zero-length). The on-chain DAO script enforces this — passing any args returns an error.

The DAO cell dep is the genesis block cell at **transaction index 0, output index 2**. It is always resolved and added to DAO transactions automatically by `ckb-cli`.

### Phase 1: Deposit (`dao deposit`)

Depositing capacity into NervosDAO locks it under the DAO type script, making it eligible for interest.

```sh
dao deposit --from-account <addr> --capacity 20000 --fee-rate 1000
```

**Arguments:**
| Argument | Description |
|---|---|
| `--from-account <lock-arg\|sighash-addr>` | Source account (required) |
| `--capacity <CKB>` | Amount to deposit in CKB (required) |
| `--fee-rate <shannons/KB>` | Transaction fee rate, default 1000 |
| `--max-tx-fee <CKB>` | Force small change as fee, max tx fee |

**What happens on-chain:**

1. A new output cell is created with:
   - **Capacity**: The deposit amount
   - **Lock script**: The user's lock script (e.g., secp256k1 sighash)
   - **Type script**: The DAO type script (`code_hash = DAO_TYPE_HASH`, `hash_type = Type`, `args = []`)
   - **Data**: 8 bytes of zeros (`[0u8; 8]`) — this marks the cell as "deposited"

2. The transaction includes the DAO cell dep (resolved from genesis block tx[0] output[2]).

3. The cell is now locked under the DAO type script. It can only be spent through the DAO prepare/withdraw process.

### Phase 2: Prepare (`dao prepare`)

The prepare step transitions a deposited cell to the "withdrawing" state. This is also called "Phase 1 withdraw" in the on-chain DAO script.

```sh
dao prepare --from-account <addr> --out-point <tx_hash>-<index> --fee-rate 1000
```

**Arguments:**
| Argument | Description |
|---|---|
| `--from-account <lock-arg\|sighash-addr>` | Source account (required) |
| `--out-point <tx_hash>-<index>` | The out-point of the deposited cell (required, can specify multiple) |
| `--fee-rate <shannons/KB>` | Transaction fee rate, default 1000 |
| `--max-tx-fee <CKB>` | Force small change as fee, max tx fee |

**What happens on-chain:**

1. Each deposited cell (data = `[0u8; 8]`) is consumed as input.

2. For each input, the builder:
   - Resolves the **deposit header** (the block where the deposit transaction was committed) by looking up the input's tx_hash.
   - Verifies the input cell has the DAO type script.

3. A corresponding output cell is created that:
   - Has the **same capacity** as the input cell
   - Has the **same lock script**
   - Has the **same DAO type script**
   - Has **data = deposit block number** (8-byte little-endian u64) — this records when the deposit occurred

4. The deposit block's header hash is added as a **header_dep** in the transaction.

5. The cell is now "prepared" (withdrawing). It remains locked for a **180-epoch** period before Phase 2 withdrawal is possible.

**Cell data transition:**
```
Deposited:  [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
Prepared:   <deposit_block_number as 8-byte LE u64>
```

### Phase 3: Withdraw (`dao withdraw`)

After the 180-epoch lock period has passed, the prepared cell can be fully withdrawn, receiving the original deposit plus accumulated interest.

```sh
dao withdraw --from-account <addr> --out-point <tx_hash>-<index> --fee-rate 1000
```

**Arguments:**
| Argument | Description |
|---|---|
| `--from-account <lock-arg\|sighash-addr>` | Source account (required) |
| `--out-point <tx_hash>-<index>` | The out-point of the prepared cell (required, can specify multiple) |
| `--fee-rate <shannons/KB>` | Transaction fee rate, default 1000 |
| `--max-tx-fee <CKB>` | Force small change as fee, max tx fee |

**What happens on-chain:**

1. Each prepared cell is consumed as input. For each input:
   - The **prepare header** is resolved (block where prepare tx was committed)
   - The cell data (8 bytes) is read to get the **deposit block number**
   - The **deposit header** is resolved from that block number
   - The **since value** is computed: `minimal_unlock_point(deposit_header, prepare_header)` calculates the earliest epoch at which the cell can be withdrawn
   - The `CellInput.since` is set to the unlock epoch (uses `SinceType::EpochNumberWithFraction`, flag `0x20`)
   - The **withdrawable capacity** is calculated using the DAO interest formula (see [Interest Calculation](#dao-interest-calculation))
   - A witness is built with the **`input_type`** field set to the **header_dep index** of the deposit block header (8 bytes LE)

2. An output cell is created that:
   - Has capacity = total withdrawable amount minus transaction fee
   - Has the user's lock script
   - Has **no type script** (the cell is freed from the DAO)
   - Has **empty data**

3. The deposit block header must be included as a header_dep.

**Key witness for withdraw:**

The first input cell's witness has the `input_type` field set to the index in `header_deps` where the deposit block header resides:

```
witness.input_type = <header_dep_index as 8-byte LE u64>
```

The on-chain DAO script reads this value to locate the deposit header for interest calculation.

### The 180-Epoch Lock Period

Cells cannot be withdrawn until a full 180-epoch period has elapsed since the deposit epoch. The `minimal_unlock_point` function computes:

```
passed_epoch_cnt = prepare_epoch - deposit_epoch
                  + (1 if prepare_epoch_fraction > deposit_epoch_fraction else 0)
unlock_epoch = deposit_epoch + ceil(passed_epoch_cnt / 180) * 180
```

The `since` on the input cell encodes this unlock epoch. The transaction is rejected as "Immature" if the current epoch hasn't reached it yet.

### DAO Interest Calculation

The interest is calculated from two block headers' `dao` fields. Each block header contains a 32-byte `dao` field with four 8-byte little-endian values:

| Bytes | Field | Description |
|---|---|---|
| 0-7 | `c` | Total capacity in the chain |
| 8-15 | `ar` | Accumulate rate — the key interest metric |
| 16-23 | `s` | Secondary issuance total |
| 24-31 | `u` | Unoccupied capacity |

The formula:

```
counted_capacity = output_capacity - occupied_capacity
withdraw_counted_capacity = counted_capacity * prepare_ar / deposit_ar
withdrawable = occupied_capacity + withdraw_counted_capacity
```

Where:
- `deposit_ar` is the `ar` value at the deposit block
- `prepare_ar` is the `ar` value at the prepare block
- `occupied_capacity` is the minimum capacity required to store the cell (61 CKB for a basic DAO cell)
- Only the "free" portion (`counted_capacity`) earns interest; the occupied portion is returned at face value

The `ar` (accumulate rate) starts at `10_000_000_000_000_000` and increases over time as secondary issuance occurs, so `prepare_ar >= deposit_ar`. The interest earned is proportional to the ratio `prepare_ar / deposit_ar`.

### Querying DAO Cells

**Query deposited cells:**

```sh
dao query-deposited-cells --address <addr>
```

Returns cells where the first 8 bytes of cell data are zero (deposited state).

**Query prepared cells:**

```sh
dao query-prepared-cells --address <addr>
```

Returns cells where the first 8 bytes of cell data are non-zero (prepared state). For each prepared cell, it additionally fetches the deposit and prepare headers to calculate and display the **maximum withdrawable amount** (including interest).

### Cell Dep Requirements Per Phase

| Phase | Required Cell Deps |
|---|---|
| **Deposit** | DAO cell dep (genesis tx[0] output[2]) |
| **Prepare** | DAO cell dep + lock script cell dep (e.g., secp256k1 sighash dep) for each input |
| **Withdraw** | DAO cell dep + lock script cell dep for each input |

### Cell Data Encoding Summary

| State | Cell Data (8 bytes) | Meaning |
|---|---|---|
| Deposited | `[0x00; 8]` | Cell is deposited, not yet prepared |
| Prepared (Withdrawing) | LE u64 block number | The block where the original deposit occurred |

### Full DAO Lifecycle Example

```sh
# 1. Deposit 20000 CKB into DAO
dao deposit --from-account ckt1qyq... --capacity 20000
# Returns: tx_hash of deposit transaction

# 2. Find the deposited cell's out-point
dao query-deposited-cells --address ckt1qyq...
# Example output: out_point = 0xe0cd7f09...-0

# 3. Prepare the cell for withdrawal (Phase 1)
dao prepare --from-account ckt1qyq... --out-point 0xe0cd7f09...-0
# Returns: tx_hash of prepare transaction

# 4. Wait at least 180 epochs (the cell must mature)

# 5. Find the prepared cell's out-point
dao query-prepared-cells --address ckt1qyq...
# Shows the maximum withdrawable amount (deposit + interest)

# 6. Withdraw the cell (Phase 2)
dao withdraw --from-account ckt1qyq... --out-point <prepare_tx_hash>-0
# Returns: tx_hash of withdraw transaction
# The capacity (original amount + interest) returns to the user's address
```

Note: `--out-point` uses the format `tx_hash-index` (e.g., `0xe0cd7f097c4e...-0`). Multiple out-points can be specified to batch prepare/withdraw multiple cells in one transaction.

---

## Contract Deployment

Uses the `deploy` subcommand with a `deployment.toml` config file.

### Workflow

1. **Initialize config**: `ckb-cli deploy init-config --deployment-config deployment.toml`
2. **Edit `deployment.toml`** with cells, dep_groups, and lock script.
3. **Generate transactions**: `ckb-cli deploy gen-txs --deployment-config ./deployment.toml --migration-dir ./migrations --from-address <addr> --sign-now --info-file info.json`
4. **Sign**: `ckb-cli deploy sign-txs --from-account <addr> --add-signatures --info-file info.json`
5. **Apply**: `ckb-cli deploy apply-txs --migration-dir ./migrations --info-file info.json`
6. **Start mining** to commit the transaction.

### `deployment.toml` Structure

```toml
[[cells]]
name = "cell_name"
enable_type_id = true    # Whether to enable type ID
location = { file = "path/to/binary" }   # Local file
# OR for on-chain reference:
# location = { tx_hash = "0x...", index = 0 }

[[dep_groups]]
name = "my_dep_group"
enable_type_id = false
cells = ["cell_name1", "cell_name2"]

[lock]
code_hash = "0x..."
args = "0x..."
hash_type = "type"

[multisig_config]        # For multisig locking
sighash_addresses = ["ckt1...", "ckt1..."]
require_first_n = 1
threshold = 2
```

- **`cells`**: Contract binaries to deploy (local file path) or reference (on-chain tx_hash + index).
- **`dep_groups`**: Groups multiple cells into a single dep_group (e.g., omni_lock + secp256k1_data).
- **`lock`**: Lock script for output cells (default is secp256k1).
- **`multisig_config`**: Multisig configuration if outputs use multisig lock.

### Migration folder structure

After deployment, the migrations folder contains one JSON file per deployment, for example:

```
{
  "cell_recipes": [
    {
      "name": "vote_type_script",
      "tx_hash": "0xd1ea1afe5d83886deaa9a4667c7a8903273bbd95522c9c6f997afdb703a12d9d",
      "index": 0,
      "occupied_capacity": 7682200000000,
      "data_hash": "0x96f66e2465eb641c5c78dc6a527593ebd85ed0000ca3f27bf4a28c5daec58f15",
      "type_id": "0x5f12cf202b50d6018eeab4e78d1f8de677fcf636b647b2f6795f66cb1f32461a"
    }
  ],
  "dep_group_recipes": []
}
```

A deployed script is referenced by its `code_hash`/`hash_type`, which maps to `<type_id>`/`"type"`. It can also be referenced as `<data_hash>`/`"data2"`, but that form is not used unless explicitly requested.

The migration folder is essential for upgrading an on-chain script: it provides the existing cell's location so the upgrade transaction can consume the old cell and create a new one.

Files are ordered by date annotated on file names; only the latest one reflects the current on-chain state.


### Note

- Before v1.8.0, addresses in `sighash_addresses` must be **short** sighash addresses.
- A migration directory stores "recipes" (JSON files with deployed cell metadata like tx_hash, index, data_hash, type_id) for tracking and future updates.
- When updating contracts, re-run `gen-txs` with updated binaries — the `type_id` remains unchanged.

---

## SUDT (Simple User-Defined Token) Operations

### Overview

SUDT is a token standard (RFC25) supporting two operations: **issue** and **transfer**. SUDT integrates with:

- **anyone-can-pay (ACP) lock**: Receive SUDT without needing CKB for capacity.
- **cheque lock**: Issue/transfer SUDT to addresses that don't have enough CKB capacity yet.

### Prerequisites

1. Deploy `simple_udt`, `anyone_can_pay`, and `ckb-cheque-script` contracts to the chain.
2. Create a `cell_deps.json` file mapping script types to their on-chain cell_dep:

```json
{
  "items": {
    "sudt": { "script_id": { "hash_type": "type", "code_hash": "0x..." }, "cell_dep": { "out_point": { "tx_hash": "0x...", "index": "0x0" }, "dep_type": "code" } },
    "acp":  { "script_id": { "hash_type": "type", "code_hash": "0x..." }, "cell_dep": { "out_point": { "tx_hash": "0x...", "index": "0x0" }, "dep_type": "dep_group" } },
    "cheque": { "script_id": { "hash_type": "type", "code_hash": "0x..." }, "cell_dep": { "out_point": { "tx_hash": "0x...", "index": "0x0" }, "dep_type": "dep_group" } }
  }
}
```

3. An `owner` account (the SUDT issuer).

### Key Commands

```sh
# Issue SUDT to a cheque address
ckb-cli sudt issue --owner <owner-addr> --udt-to <receiver>:<amount> --to-cheque-address --cell-deps ./cell_deps.json

# Issue SUDT directly to an ACP address
ckb-cli sudt issue --owner <owner-addr> --udt-to <acp-addr>:<amount> --to-acp-address --cell-deps ./cell_deps.json

# Create an empty ACP cell for receiving SUDT
ckb-cli sudt new-empty-acp --owner <owner-addr> --to <receiver> --cell-deps ./cell_deps.json

# Transfer SUDT from sender ACP address
ckb-cli sudt transfer --owner <owner-addr> --sender <sender-acp-addr> --udt-to <target>:<amount> --to-acp-address|--to-cheque-address --cell-deps ./cell_deps.json

# Claim SUDT from a cheque (sender created a cheque, receiver claims it)
ckb-cli sudt cheque-claim --owner <owner-addr> --sender <sender-addr> --receiver <receiver-addr> --cell-deps ./cell_deps.json

# Withdraw SUDT from a cheque (receiver didn't claim in time, sender withdraws back)
ckb-cli sudt cheque-withdraw --owner <owner-addr> --sender <sender-addr> --receiver <receiver-addr> --to-acp-address --cell-deps ./cell_deps.json

# Query SUDT amount for an address
ckb-cli sudt get-amount --owner <owner-addr> --address <addr> --cell-deps ./cell_deps.json
```

### SUDT Workflows

- **Issue via cheque**: `sudt issue --to-cheque-address` -> receiver creates empty ACP cell (`sudt new-empty-acp`) -> receiver claims (`sudt cheque-claim`).
- **Issue directly to ACP**: `sudt issue --to-acp-address` (capacity must be provided by the owner).
- **Transfer via cheque**: `sudt transfer --to-cheque-address` from an ACP address -> receiver claims -> capacity transfers to receiver's ACP.
- **Cheque withdrawal**: If sender's cheque is not claimed within ~6 epochs, sender can withdraw back using `sudt cheque-withdraw`.

---

## Neuron Wallet Interoperability

### Import ckb-cli keystore to Neuron

1. Copy keystore: `cp ~/.ckb-cli/keystore/UTC--<timestamp>--<lock_arg> tmp-keystore.json`
2. Edit the JSON: remove fields `ckb_root`, `origin`, `hash160`; keep only `version`, `id`, `crypto`.
3. In Neuron: `Wallet > Import Wallet > Import from Keystore`.

### Import Neuron wallet to ckb-cli

```sh
ckb-cli account import-keystore --path "Wallet x.json"
```

### Key Usage Differences

- **ckb-cli**: Uses master key (path `m`) as primary key (shown in `account list`). Also supports BIP44 (path `m/44'/309'/0'`).
- **Neuron**: Only uses BIP44 address space (path `m/44'/309'/0'`).

---

## Usage Examples

### Transfer Capacity

```sh
# Query capacity
wallet get-capacity --address ckt1qyqrq6m2spk7rz8mksz70dp92lcmnrre03dsqrfdlh

# Transfer
wallet transfer --from-account <addr> --to-address <addr> --capacity 20000 --max-tx-fee 0.00001

# Check transaction status
rpc get_transaction --hash 0x...
```

### Timelocked Address Workflow

```sh
# 1. Build timelocked multisig address
util to-multisig-addr --sighash-address <addr> --locktime "2019-12-06T18:00:00+00:00"

# 2. Transfer capacity into the timelocked address
wallet transfer --from-account <addr> --to-address <timelocked-addr> --capacity 20000

# 3. After unlock epoch, transfer out
wallet transfer --from-account <normal-addr> --from-locked-address <timelocked-addr> --to-address <dest> --capacity 19999.9999 --tx-fee 0.0001
```

For BIP32 derived keys, add `--derive-change-address <change-addr>`.

### Epoch-Based Multisig Address with Since

```sh
# Build address locked until epoch 61
tx build-multisig-address --sighash-address <addr> --since-absolute-epoch 61

# Transfer to it, then create tx.json, add input, add output, sign, send
tx init --tx-file tx.json
tx add-multisig-config --multisig-code-hash legacy --sighash-address <addr> --tx-file tx.json
tx add-input --tx-hash <hash> --index 0 --tx-file tx.json
tx add-output --to-sighash-address <dest> --capacity 19999.9999 --tx-file tx.json
tx sign-inputs --from-account <key> --add-signatures --tx-file tx.json
tx send --tx-file tx.json
```

## Configuration

Set `CKB_CLI_HOME` to override the default `~/.ckb-cli` directory. Config files and keystores are stored in this folder.



