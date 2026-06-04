# CKB Devnet Node

## Overview

CKB Devnet is a local, isolated blockchain environment for fast development and testing. It does not require syncing the full chain.

Two worker types:
- **Dummy-Worker** (recommended): constant block intervals, no PoW — fast and predictable
- **Eaglesong-Worker**: real PoW mining — use only when validating PoW; block times can be erratic at low hashrate

## Setup

### Download

Get the CKB binary from [GitHub Releases](https://github.com/nervosnetwork/ckb/releases). Unzip and navigate into the directory:

```sh
cd /path/to/ckb_v0.206.0_aarch64-apple-darwin-portable/ckb
./ckb --version
./ckb-cli --version
```

### Dummy-Worker Setup

**1. Initialize**

```sh
./ckb init --chain dev
```

Creates `specs/dev.toml`, `ckb.toml`, `ckb-miner.toml`, and prints the genesis hash.

**2. Configure Block Assembler**

```sh
./ckb-cli account new    # record the lock_arg value
```

In `ckb.toml`, set the `block_assembler` section:

```toml
[block_assembler]
code_hash = "0x9bd7e06f3ecf4be0f2fcd2188b23f1b9fcc88e5d4b65a8637b17723bbda3cce8"
args = "<your lock_arg>"
hash_type = "type"
message = "0x"
```

**3. Optional: Shorten Block Interval**

In `specs/dev.toml`:

```toml
[params]
genesis_epoch_length = 10          # default 1000
permanent_difficulty_in_dummy = true
```

In `ckb-miner.toml`:

```toml
[[miner.workers]]
worker_type = "Dummy"
delay_type = "Constant"
value = 5000                       # ms, default 5000 (5s)
```

**4. Start Node**

```sh
./ckb run
```

**5. Start Miner** (separate terminal)

```sh
./ckb miner
```

### Eaglesong-Worker Setup

**1. Create account:**

```sh
./ckb-cli account new
```

**2. Initialize with block assembler arg:**

```sh
./ckb init -c dev --ba-arg <lock_arg>
```

**3. In `specs/dev.toml`**, set `pow.func = "Eaglesong"`.

In `ckb-miner.toml`:

```toml
[[miner.workers]]
worker_type = "EaglesongSimple"
threads = 1
```

**4. Start node:** `./ckb run`

**5. Start miner:** `./ckb miner`

## Key Configuration Files

| File | Purpose |
|------|---------|
| `specs/dev.toml` | Chain spec: PoW function (`Dummy` / `Eaglesong`), epoch length, difficulty params |
| `ckb.toml` | Block assembler (`code_hash`, `args`, `hash_type`, `message`) |
| `ckb-miner.toml` | Miner workers (`worker_type`, `delay_type`, `value`, `threads`) |

## Genesis Issued Cells

Pre-funded cells created by `ckb init --chain dev`, available only on the local devnet:

| Cell | Private Key | Lock Arg | Capacity |
|------|-------------|----------|----------|
| #1 | `0xd00c06bfd800d27397002dca6fb0993d5ba6399b4238b2f29ee9deb97593d2bc` | `0xc8328aabcd9b9e8e64fbc566c4385c3bdeb219d7` | 20,000,000,000 CKB |
| #2 | `0x63d86723e08f0f813a36ce6aa123bb2289d90680ae1e99d4de8cdb334553f24d` | `0x470dcdc5e44064909650113a274b3b36aecb6dc7` | 5,198,735,037 CKB |

Import them:

```sh
echo 0xd00c06bfd800d27397002dca6fb0993d5ba6399b4238b2f29ee9deb97593d2bc > pk1
echo 0x63d86723e08f0f813a36ce6aa123bb2289d90680ae1e99d4de8cdb334553f24d > pk2
./ckb-cli account import --privkey-path pk1
./ckb-cli account import --privkey-path pk2
```

## Using ckb-cli

The `ckb-cli` tool provides interactive RPC access for account management, transfers, and balance checks. **For development/testing only** — use a wallet for real funds.

```sh
./ckb-cli            # enter interactive mode

# Create account
account new

# Check balance
wallet get-capacity --address <address>

# Transfer
wallet transfer --from-account <addr> --to-address <addr> --capacity 10000 --max-tx-fee 0.00001

# Import genesis accounts
account import --privkey-path pk1
```
