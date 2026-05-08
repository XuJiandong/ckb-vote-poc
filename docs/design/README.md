# Voting Design and implementation with zkVM on CKB-VM

This document explains how to use a zkVM to design and implement a voting system on CKB-VM. Any zkVM (SP1, RISC Zero, etc.) can be used;
here we use SP1 since we have already ported the SP1 verifier to CKB-VM.

## Introduction

Before diving into the implementation details, let's cover some basic concepts about what a zkVM does and how it works.

We define the *guest program* as the program running inside the zkVM. It can output public values to the *host program*.
The host program runs outside the zkVM and is responsible for verifying the proof and receiving those public values.

Using a zkVM, we can prove that the following statement is true:

```
Given a start block hash and an end block hash, there exists a cell satisfying a specific condition.
```

For example, between two blocks, we can verify that there is a cell with a specific type script whose cell data contains
exactly 1 byte with value `0xFF`.

**Guest program workflow:**

1. Read blocks sequentially, starting from `<start block hash>` and ending at `<end block hash>`.
2. Verify that `parent_hash` matches between adjacent blocks.
3. Verify that the `transactions_root` field in each block matches what is expected according to the [block structure RFC](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0027-block-structure/0027-block-structure.md). Additional verification steps can be included as needed, but we do not detail them here.
4. Parse all blocks in molecule format, read all transactions, iterate over all cells, and find the target cell.
5. Verify that the cell meets the required condition.
6. Commit (output): `<start block hash>`, `<end block hash>`: there will be in proof.
7. Generate the proof.

**Host program verification:**

1. The proof is valid.
2. Read `<start block hash>` and `<end block hash>` from proof, and verify it matches the values we provided.

Note: there is no need to execute all lock scripts and type scripts. Because the block hashes are verified, it is impossible
for the guest to tamper with the block data — if the data fed to the guest is corrupted, the guest program will fail to complete.
Overall, this approach is efficient, as it only involves block hashing and parsing operations—there are no intensive cryptographic computations like secp256k1 signing or script validation. Additionally, since CKB-VM scripts are not executed during this process, ckb-vm itself does not run, further reducing computational overhead.

Using this approach, we can do many interesting things, including implementing a voting system. The implementation is straightforward:
write a utility that reads, parses, and verifies.


## Before We Start

This document focuses on the design and partial implementation of a voting system using zkVM on CKB-VM. It is not a full specification and does not cover every implementation detail.

## Proposal Cell

A proposal cell is the central element of the design. It can only be created by designated individuals or a team — not by arbitrary users(via owner mode). It represents a proposal, and once it appears on-chain, voting begins. Users can cast votes in response to the proposal.

The lock script of a proposal cell is always a success lock script. All access control is delegated to the type script described below.

The type script of a proposal cell is called the proposal type script. Its `args` are defined as:

```text
<20-byte blake160 hash of previous TX> <20-byte blake160 hash of owner lock script> <32-bytes SP1 verifying key hash>
```

The first 20 bytes ensure uniqueness via the Type ID mechanism (see [Type ID implementation](https://github.com/nervosnetwork/ckb-std/blob/0a16c0ed8a6b4d8194d64420dbe309a0c23fc1b2/src/type_id.rs#L79-L85)).
Then the following 20 bytes serve to enable the `owner mode` mechanism. To determine if `owner mode` is active, the script scans all lock scripts within the current transaction and checks whether the specified owner lock is also successfully unlocked. If so, the script operates in `owner mode`. Typically, these owner lock scripts are associated with recognized individuals or trusted teams. The owner lock script list should be available publicly.
The final 32 bytes represent the SP1 verifying key hash, indicating which SP1 guest program should be used for zkVM proof verification.

It must follow these rules:

* Proposal cell can only be created under `owner mode`
* It must be unique across all type scripts. This can be achieved using Type ID.
* It is referenced by vote cells via its Type ID.
* It validates the cell data format, described below.
* If a proposal cell remains on-chain without being destroyed for a long time, it indicates the proposal did not pass. It can be recycled after a specific expiration time.
* A proposal cell can only be created or destroyed, not updated. 
* When a proposal cell is destroyed, a zkVM proof must be provided to demonstrate that the vote passed. Only a successful vote permits destruction. The proof is stored in the corresponding witness. This step is described in a separate section (zkVM Verifying Process) and is the most critical part of the design.
* When the zkVM proof is verified, there must be an output cell with the `amount` and a lock script matching the `receiver`, as described below. This output represents the funds released when the proposal passes.
* Destroying a proposal cell via the `owner mode` mechanism bypasses all other rules.

The proposal cell data format includes the following fields:

1. `duration` (N) in blocks: votes are valid only if cast within N consecutive blocks from the proposal's start. Votes outside this range are not counted.
2. `vote cell code_hash / hash_type`: specifies the script a vote cell must use. Cells using a different script are not counted as valid votes.
3. `expired_time`: after this time, the proposal cell can be recycled by the designated individuals or team.
4. `description`: a plain-text description of the proposal.
5. `receiver`: The address that will receive the CKB amount when the proposal passes.
6. `amount`: The amount of CKB to be received.
7. `minimal_requirement`: minimum required CKB involved in voting.

Other metadata may also be included; only the important fields are listed here.

Design notes and rationale:

* Why can only designated individuals or the team create proposals? They are involved only once — during the creation of the proposal cell. After that, anyone can operate on the related cells, including generating zkVM proofs. Allowing anyone to create proposals would open the system to DDoS attacks.
* Allowing updates would require revoking old votes, re-voting, and notifying all participants — an impractical workflow. The recommended approach is to abandon the existing proposal and create a new one. The "owner mode" mechanism can be used to destroy it.
* Only authorized lock scripts may operate in `owner mode` to withdraw funds from the treasury. While other users can participate in the voting system, only designated entities with the authorized lock script are permitted to access or withdraw treasury funds.
* Third parties may also utilize this voting system. However, they are not permitted to withdraw funds from the treasury.

## Vote Cell

Once the proposal cell is on-chain, users can cast their votes by creating vote cells. Each vote cell's data contains the vote result — typically a simple yes or no.

The vote cell's lock script can be anything, as users may recycle these cells immediately after voting. In practice, it is recommended to use the same lock script as the input cell.

The vote cell's type script is specified in the proposal cell's data format. This vote type script must follow these rules:

1. The input cell being consumed must be unlocked by a script. This script represents ownership of a DAO deposit.
2. `cell_deps` must include a reference to the DAO deposit, which must share the same lock script as above. The DAO deposit amount is used as the vote weight.
3. `args` must contain the Type ID of the proposal cell. Since each proposal type script is unique, all derived vote type scripts are unique as well.
4. Rules 1–3 apply only when this type script appears in an output cell. When it appears in an input cell, it does nothing — this allows the cell to be recycled.

Design notes and rationale:

1. The type script is not required to validate the proposal cell in full. It only checks that the proposal cell exists. Full validation is handled by the zkVM verifying process.
2. Vote cells outside the proposal's `duration` range are not counted as valid votes. This is also enforced in the zkVM verifying process.
3. Vote cells can be recycled immediately, so they do not lock up users' capacity. Users only pay a small amount of CKB as transaction fees.

## Treasury Cell
The treasury cell holds assets by default. When a proposal passes, anyone can generate a zkVM proof to unlock the proposal type script.
The treasury cell is locked by a special treasury lock script, which can be unlocked when the proposal type script in the same transaction is also unlocked.
The transaction includes treasury cells and a proposal cell, with an output cell using the `receiver` as the lock script — effectively sending funds to the receiver.

The treasury lock script follows these rules:

1. The proposal cell in the same transaction must have a valid `code_hash` / `hash_type`.
2. The `args` of the proposal cell must be valid:
   - The owner lock script hashes are recognized.
   - The verifying key hash matches.
   These two settings are critical to the voting system. The owner lock script hashes determine who can initiate a proposal, and the verifying key hash defines the shape of the guest program — it must be updated whenever the guest program changes. Allowing these settings to be malformed or overwritten by anyone would be a serious security issue.


Design notes and rationale:

1. The owner lock script hashes can be hard-coded in the treasury lock script. The logic is simple and easy to update.
2. The verifying key hash can also be updated when the guest program changes.


## zkVM Verifying Process

This is the most critical part of the design. It consists of two sides: off-chain and on-chain.
The off-chain side generates the SP1 proof using block data as input. The on-chain side verifies the proof against the real block data.

On the off-chain side, the guest program works as follows:

- The guest program receives, as input arguments, a sequence of block data beginning with the block containing the proposal cell. This sequence consists of exactly `duration + 1` consecutive blocks.
- It also receives a collection of relevant transactions associated with valid DAO deposits, organized in a map structure, as additional input arguments.
- The guest program takes last argument as the proposal type script along with its corresponding cell data.
- It reads the proposal cell from the first block and retrieves the vote cell type script.
- It verifies that `parent_hash` matches between adjacent blocks.
- It verifies that the `transactions_root` field in each block matches the expected value according to the [block structure RFC](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0027-block-structure/0027-block-structure.md). Additional verification steps can be included as needed, but are not detailed here.
- It parses all blocks in molecule format, reads all transactions, and iterates over all cells.
- If a cell matches the vote cell type script, it verifies that the corresponding `cell_deps` contain a DAO deposit with the correct lock script. If valid, the vote is counted and stored in a `Map`. Later votes overwrite earlier ones from the same voter.
- After the final block is processed, a `Map` of voting results is produced. The key is the lock script hash identifying each voter, and the value is the amount of CKB they hold.
- The passing rule is not yet finalized, but a simple example would be: `sum("YES") > sum("NO") && sum("YES") + sum("NO") > minimal_requirement`.
- Finally, the guest program commits the following public values and outputs the SP1 proof:
    * Start block hash
    * End block hash
    * Proposal type script and its cell data
    * Whether the proposal passed
  These public values are crucial, as the on-chain verifier must independently validate each of them to ensure the integrity of the zkVM proof.

On the on-chain side, the proposal type script verifies the SP1 zkVM proof as follows:

1. Read the `<32-byte SP1 verifying key hash>` from `args`.
2. Read the `proof` from the witness.
3. Parse the `proof` to extract the public values.
4. Call the SP1 verifier with the arguments above and verify the proof.
5. Verify that the public values match the on-chain data:
   - The start block hash must match the first `header_dep`.
   - The end block hash must match the second `header_dep`.
   - The proposal type script and its cell data must match.
   - The proposal passing flag must match.

Note: the start and end block hashes are referenced via `header_dep`. If either hash is invalid, the reference will fail and the transaction cannot be constructed.

## Benchmark and Optimization

We benchmarked the solution against 500 mainnet blocks. The total cost is approximately 61M cycles, broken down into two categories:

1. `verify_transaction_root`: The most expensive step. It computes all transaction hashes, builds a Merkle tree, and then derives the transaction root — costing about 55M cycles.
2. Other checks (block header hash verification, cell traversal, etc.): about 6M cycles.

At this rate, processing one day's worth of blocks (assuming one block every 10 seconds) would cost roughly 1000M cycles, which is practically infeasible.

### Optimized approach: count "YES" votes only

An alternative is to only count "YES" votes and disallow "NO" votes. Voting passes once the "YES" count exceeds a threshold. This allows skipping `verify_transaction_root` for blocks that contain no vote data. Since attackers cannot hide specific vote cells to manipulate the outcome, a valid proof can always be constructed by including sufficient vote cells up to the threshold — significantly reducing proof generation time.

As a rough estimate, assuming 50 blocks per day contain vote cells, the guest program cost drops to about 103M cycles — a 90% reduction.

```
5.5M (transaction_root) + 6/500 * 3600*24/10 (others) = 103M (cycles)
```

## Diagrams

The following diagram shows the static structure of the three core cell types and how they interact through the zkVM verifying process:

![zkVM Voting System — Cell Structures & Verification Process](./zkvm-voting.png)


The diagram below shows the block timeline: `duration + 1` consecutive blocks (Block 0 through Block N) contain the proposal cell and vote cells, while the final transaction — which consumes the proposal cell and treasury cell — lives in a later block outside that window.

![zkVM Voting — Block Timeline & Final Transaction](./zkvm-voting-blocks.png)

The diagram below gives a bird's-eye view of the entire voting lifecycle across all four phases and actors. 

![zkVM Voting — End-to-End Lifecycle](./zkvm-voting-flow.png)

