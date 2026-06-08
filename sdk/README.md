# CKB Vote SDK

Off-chain TypeScript SDK for the CKB on-chain voting system. Provides an API and CLI to create proposals, cast votes, and settle passed proposals.

Built on [CCC](https://github.com/ckb-devrel/ccc) (`@ckb-ccc/shell`).

## Installation

```sh
cd sdk
pnpm install
```

## CLI Usage

Run directly without a build step:

```sh
./node_modules/.bin/tsx src/cli/index.ts <command> [options]
# or after `pnpm link --global`:
ckb-vote <command> [options]
```

### Quick Start (devnet)

```sh
# Create a proposal (20-block window)
pnpm dev create-proposal \
  --private-key-file ../tools/e2e/pk1 \
  --duration 20 \
  --description "test1"

# Vote YES on the proposal
pnpm dev vote \
  --private-key-file ../tools/e2e/pk1 \
  --proposal-tx-hash 0x<TX_HASH_FROM_ABOVE> \
  --vote yes

# Consume / settle after proof is generated
pnpm dev consume-proposal \
  --private-key-file ../tools/e2e/pk1 \
  --proposal-tx-hash 0x<TX_HASH_FROM_ABOVE> \
  --proof ./proof-plonk.bin \
  --public-values ./public-values.bin \
  --start-block-hash 0x<START> \
  --end-block-hash 0x<END>
```

All commands share a common set of options:

- `--private-key-file <path>` — path to a file containing your 32-byte hex private key
- `--rpc-url <url>` — CKB RPC endpoint (default: `http://127.0.0.1:8114`)

---

### `create-proposal`

Create a proposal cell on-chain. The proposal type script args (Type ID + SP1 verifying key) are computed automatically.

```sh
ckb-vote create-proposal \
  --private-key-file ./my-key.txt \
  --duration 8640 \
  --description "Fund infrastructure work Q3 2026" \
  [--receiver ckt1qzda0cr...] \
  [--amount 1000] \
  [--minimal-requirement 5000] \
  [--rpc-url http://127.0.0.1:8114]
```

| Option                  | Required | Description                                                            |
| ----------------------- | -------- | ---------------------------------------------------------------------- |
| `--private-key-file`    | Yes      | Path to hex private key file                                           |
| `--duration`            | Yes      | Voting window in blocks (~10s/block)                                   |
| `--description`         | Yes      | Plain-text description                                                 |
| `--receiver`            | No       | CKB address for funds if proposal passes; defaults to signer's address |
| `--amount`              | No       | CKB to transfer on success; defaults to 0                              |
| `--minimal-requirement` | No       | Minimum CKB vote weight to pass; defaults to 0                         |

Output:

```
Proposal created successfully.
  tx hash:  0x...
  outpoint: 0x...:0
```

---

### `vote`

Cast a YES or NO vote on a proposal. All DAO deposit cells owned by the signer are discovered automatically and recorded in `dao_index`.

```sh
ckb-vote vote \
  --private-key-file ./my-key.txt \
  --proposal-tx-hash 0xABC... \
  --vote yes \
  [--proposal-index 0] \
  [--rpc-url http://127.0.0.1:8114]
```

| Option               | Required | Description                                    |
| -------------------- | -------- | ---------------------------------------------- |
| `--private-key-file` | Yes      | Path to hex private key file                   |
| `--proposal-tx-hash` | Yes      | Tx hash of the proposal cell                   |
| `--vote`             | Yes      | `yes` or `no`                                  |
| `--proposal-index`   | No       | Output index of the proposal cell (default: 0) |

Output:

```
Vote (yes) submitted successfully.
  tx hash:    0x...
  vote cell:  0x...:0
```

---

### `consume-vote`

Recycle a vote cell at any time to reclaim its occupied CKB.

```sh
ckb-vote consume-vote \
  --private-key-file ./my-key.txt \
  --vote-tx-hash 0xABC... \
  [--vote-index 0]
```

---

### `consume-proposal`

Settle a passed proposal by submitting the SP1 PLONK proof. The proof and public values are binary files produced by the SP1 prover.

```sh
ckb-vote consume-proposal \
  --private-key-file ./my-key.txt \
  --proposal-tx-hash 0xABC... \
  --proof ./proof-plonk.bin \
  --public-values ./public-values.bin \
  --start-block-hash 0xSTART... \
  --end-block-hash 0xEND... \
  [--proposal-index 0]
```

| Option               | Required | Description                                        |
| -------------------- | -------- | -------------------------------------------------- |
| `--proof`            | Yes      | Path to SP1 PLONK proof binary file                |
| `--public-values`    | Yes      | Path to molecule-encoded PublicValues binary file  |
| `--start-block-hash` | Yes      | Hash of the start block (becomes `header_deps[0]`) |
| `--end-block-hash`   | Yes      | Hash of the end block (becomes `header_deps[1]`)   |

---

## API Usage

```typescript
import { ccc } from "@ckb-ccc/shell";
import {
  buildClient,
  createProposal,
  createVote,
  consumeProposal,
  DEVNET_CONFIG,
} from "@ckb-vote/sdk";

const client = buildClient("http://127.0.0.1:8114");
const signer = new ccc.SignerCkbPrivateKey(client, "0xYOUR_PRIVATE_KEY");

// Create a proposal
const { txHash, proposalOutPoint } = await createProposal(signer, {
  duration: 8640,
  description: "Fund Q3 infrastructure work",
  amount: 1000n * 100_000_000n, // 1000 CKB in shannon
  minimalRequirement: 5000n * 100_000_000n,
});

// Vote YES (DAO deposits are auto-discovered)
const voteResult = await createVote(signer, {
  proposalOutPoint,
  vote: "yes",
});

// Consume / settle (after proof is generated)
import { readFileSync } from "fs";
const { txHash: settleTx } = await consumeProposal(signer, {
  proposalOutPoint,
  proof: "0x" + readFileSync("proof-plonk.bin").toString("hex"),
  publicValues: "0x" + readFileSync("public-values.bin").toString("hex"),
  startBlockHash: "0xSTART_HASH...",
  endBlockHash: "0xEND_HASH...",
});
```

---

## Network Configuration

The SDK ships with `DEVNET_CONFIG` for the local devnet. To use a different network, pass a `config` override to any API function:

```typescript
import { DEVNET_CONFIG, type NetworkConfig } from "@ckb-vote/sdk";

const testnetConfig: NetworkConfig = {
  ...DEVNET_CONFIG,
  ckbRpcUrl: "https://testnet.ckb.dev",
  // override script deployments for testnet:
  proposalTypeScript: {
    codeHash: "0xTESTNET_CODE_HASH...",
    hashType: "type",
    outPoint: { txHash: "0xTESTNET_TX...", index: 0 },
  },
  // ... other overrides
};

await createProposal(signer, { ..., config: testnetConfig });
```

For the CLI, supply `--rpc-url` to override the endpoint; the devnet script deployment info is always used by default.

---

## Devnet Script Info

| Script               | Code Hash       | Tx Hash         | Index |
| -------------------- | --------------- | --------------- | ----- |
| always-success       | `0xfb9026d2...` | `0x2cccea9e...` | 0     |
| proposal-type-script | `0xa5bf702b...` | `0x5e30bf87...` | 0     |
| vote-type-script     | `0xf659306b...` | `0x3d4d939b...` | 0     |

SP1 verifying key: `0x00eed8303c92b34e9bf35c4e3424436d4b9afcc38637a1633c000a90cb54bf7a`

## Transaction Structure

### Create Proposal

```
Inputs:   [signer cells]
Outputs:  [0] Proposal cell
              lock: always-success
              type: proposal-type-script (args = blake160(typeId) || sp1VkHash)
              data: molecule-encoded Proposal
Cell deps: proposal-type-script contract, always-success contract
```

### Vote

```
Cell deps: [0] vote-type-script contract
           [1] proposal cell
           [2..] DAO deposit cells  ← dao_index points here
Inputs:   [signer cells for fee]
Outputs:  [0] Vote cell
              lock: voter's lock
              type: vote-type-script (args = blake160(proposalTypeScript))
              data: molecule-encoded Vote {vote, amount, dao_index}
```

### Consume Proposal

```
Inputs:   [0] Proposal cell (always-success lock, witness in input_type)
          [signer cells for fee]
Header deps: [startBlockHash, endBlockHash]
Outputs:  [0] Receiver cell (Proposal.amount capacity, Proposal.receiver lock)
Cell deps: proposal-type-script contract, always-success contract
Witness:  WitnessArgs { input_type: ProposalWitness { proof, public_values } }
```
