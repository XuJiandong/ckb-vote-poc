# Proposal Type Script Specification

A proposal type script is the type script used in the proposal cell described [here](./design/README.md).

A proposal cell represents a proposal, and once it appears on-chain, voting begins. Users can cast votes in response to the proposal.

## Script

The basic type script structure is as follows:

```text
code_hash: <proposal type script code_hash>
hash_type: <proposal type script hash_type>
args: <20-byte blake160 hash of Type ID> <32-bytes SP1 verifying key hash>
```

The first 20 bytes of args ensure uniqueness via the Type ID mechanism (see [Type ID implementation](https://github.com/nervosnetwork/ckb-std/blob/0a16c0ed8a6b4d8194d64420dbe309a0c23fc1b2/src/type_id.rs#L79-L85)).
The final 32 bytes represent the [SP1 verifying key](https://docs.zkverify.io/architecture/verification_pallets/sp1), indicating which SP1 guest program should be used for zkVM proof verification.

The corresponding lock script in the proposal cell should be an always-success lock script. At a later stage, this cell can be consumed by anyone who provides the SP1 proof.

When a proposal cell is created, the proposal type script appears in the output cells. When consumed, it appears in the input cells.
It is not allowed to appear on both sides, to prevent updating an existing proposal cell.

## Witness

When a proposal cell is created, no witness is required. When a proposal cell is consumed, a witness must be provided at the corresponding input position:

```
WitnessArgs:
    lock: <..>
    input_type: <..>
    output_type: <ProposalWitness>
```

`ProposalWitness` has the following molecule structure:

```
// witness layout of proposal cell
table ProposalWitness {
    proof: Bytes,
    public_values: PublicValues,
}
```

## Cell Data

**Subject to change if requirements change.**

It is a molecule structure as follows:

```
table Proposal {
    duration: Uint32,
    vote_cell_code_hash: Byte32,
    vote_cell_hash_type: byte,
    description: Bytes,
    receiver: Script,
    amount: Uint64,
    minimal_requirement: Uint64,
}
```

1. `duration` (N) in blocks: votes are valid only if cast within N consecutive blocks from the proposal's start. Votes outside this range are not counted.
2. `vote cell code_hash / hash_type`: specifies the script a vote cell must use. Cells using a different script are not counted as valid votes.
3. `description`: a plain-text description of the proposal, in UTF-8 format.
4. `receiver`: the address that will receive the CKB amount when the proposal passes.
5. `amount`: the amount of CKB to be received.
6. `minimal_requirement`: minimum required CKB involved in voting.

Since proposal cells can be created by anyone, the fields `duration`, `vote cell code_hash/hash_type`, `amount`, and `minimal_requirement` must be constrained by the proposal type script. These parameters will be published once the voting system is complete.


## Unlocking Process
### Creation
When a proposal cell is created (the type script is on the output side), the script must verify the following:

1. The 20-byte blake160 hash of the Type ID in `args` matches.
2. The following fields are validated:
   - The 32-byte SP1 verifying key hash in `args`
   - Vote cell `code_hash` / `hash_type` in cell data
   - `duration` in cell data
   - `amount` in cell data
   - `minimal_requirement` in cell data
3. There should be only one such type script in transaction.

The SP1 verifying key hash and vote cell `code_hash` / `hash_type` are updated when the guest program is compiled and the vote type script is deployed. The remaining fields are under discussion.

### Consuming
First, the script reads `ProposalWitness` from the witness, in the `WitnessArgs.output_type` field, and extracts the following items:
- proof
- public_values

It then reads the `verifying_key` from args as 32 bytes and verifies using:

```rust
PlonkVerifier::verify(&proof, &public_values, &verifying_key, sp1_verifier::PLONK_VK_BYTES)
```
The modified `sp1_verifier` is [here](https://github.com/XuJiandong/sp1).

The public values are encoded as a molecule structure as follows:

```
table PublicValues {
    proposal: Proposal,
    start_block_hash: Byte32,
    end_block_hash: Byte32,
    proposal_script: Script,
    passed: byte,
    yes_vote: Uint64,
    no_vote: Uint64,
}
```

* The type script verifies that the `proposal` field matches the cell data.
* It verifies that `header_deps[0]` and `header_deps[1]` correspond to `start_block_hash` and `end_block_hash` respectively. These hashes are referenced via `header_dep`; if either is invalid, the reference fails and the transaction cannot be constructed.
* It verifies that `proposal_script` matches the current type script.
* Finally, it verifies that `passed` is `1`.

If all checks pass, the type script unlocks.

## Guest Program & zkVM Verifying Process
This is the most critical part of the design. It consists of two sides: off-chain and on-chain.
The off-chain side generates the SP1 proof using block data as input. The on-chain side verifies the proof.

The verifying key in `args` is a hash of the guest program. The guest program performance following tasks in zkVM:

- The guest program receives, as input arguments, a sequence of block data beginning with the block containing the proposal cell. This sequence consists of exactly `duration + 1` consecutive blocks.
- The guest program takes last argument as the proposal type script.
- It reads the proposal cell from the first block and retrieves the vote cell type script. The guest program cannot choose a favorable block range: the start block hash and duration are committed as public values and verified on-chain. The end block hash is also committed to prove that the end block exists on-chain.
- It verifies that `parent_hash` matches between adjacent blocks.
- It verifies that the `transactions_root` field in each block matches the expected value according to the [block structure RFC](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0027-block-structure/0027-block-structure.md). Additional verification steps can be included as needed, but are not detailed here.
- It parses all blocks in molecule format, reads all transactions, and iterates over all cells.

- If a cell matches the vote type script, the verifier checks that its cell data contains a valid `table Vote` (see [Vote Type Script Specification](./vote-type-script.md#cell-data)). The `Vote.amount` field is recorded in a `Map` keyed by the voter's lock script hash. The `Map` allows no duplicate keys, so inserting an entry for an existing key overwrites the previous value. This means a later vote from the same voter replaces the earlier one, effectively allowing vote retraction.

- Each `table Vote` carries a `dao_index`. The corresponding DAO deposit out points from `cell_deps` are added to a `Map2` keyed by out point, with the voter's lock script hash as the value. If a subsequent transaction spends an out point already in `Map2`, it indicates the same CKB would be counted twice; the associated voter is looked up in `Map2` and their entry is removed from both `Map2` and `Map`, preventing double-counting of DAO deposits.

- After the final block is processed, the voting results are aggregated into a `Map`. The key is the lock script hash identifying each voter, and the value is the total CKB amount they hold.
- The passing rule is not yet finalized, but a simple example would be: `sum("YES") > sum("NO") && sum("YES") + sum("NO") > minimal_requirement`.
- Finally, the guest program commits the public values(See `table PublicValues`) and outputs the SP1 proof. These public values are crucial, as the on-chain verifier(see above) must independently validate each of them to ensure the integrity of the zkVM proof.

## Other Notes

* It is deliberately designed that if a proposal fails, no one can recycle the cells. This serves as a penalty to discourage flooding the system with proposals.

* Allowing updates would require revoking old votes, re-voting, and notifying all participants — an impractical workflow. The recommended approach is to abandon the existing proposal and create a new one. 

* Third parties may also utilize this voting system. The can refer existing proposal type script.

* For vote-time eligibility: a DAO deposit created during the voting window can be used to vote. This means a user can buy CKB on the market and deposit it into the DAO to gain voting rights. Since the deposit will be locked for a period, this design encourages broader DAO participation.


## Examples

### Example 1: Creating a Proposal Cell

```yaml
Inputs:
    <any> Funding_Cell
        Data: <empty>
        Type: <none>
        Lock:
            <proposer's lock script>

Outputs:
    Proposal_Cell
        Data:
            Proposal (molecule):
                duration: 8640                          # ~1 day (8640 blocks × ~10s)
                vote_cell_code_hash: <32-byte hash of vote type script>
                vote_cell_hash_type: 0x01               # type
                description: "Fund infrastructure work Q3 2026"
                receiver:
                    code_hash: <secp256k1 code hash>
                    hash_type: 0x01                     # type
                    args: <20-byte blake160 of receiver pubkey>
                amount: 1000                            # 1000 CKB
                minimal_requirement: 5000               # 5000 CKB total vote weight
        Type:
            code_hash: <proposal type script code_hash>
            hash_type: <proposal type script hash_type>
            args:
                <20-byte blake160 Type ID>              # blake160(first_input_out_point || output_index)
                <32-byte SP1 verifying key hash>
        Lock:
            code_hash: <always-success lock code_hash>
            hash_type: <always-success lock hash_type>
            args: <empty>

    <any> Change_Cell
        Data: <empty>
        Type: <none>
        Lock:
            <proposer's lock script>

Witnesses:
    WitnessArgs structure:
        Lock: <proposer's signature>
        Input Type: <none>
        Output Type: <none>                             # no witness needed on creation
```

---

### Example 2: Consuming a Proposal Cell (Proposal Passed)

```yaml
Inputs:
    Proposal_Cell                                       # index 0 — witness position matches
        Data:
            Proposal (molecule):
                duration: 8640
                vote_cell_code_hash: <32-byte hash of vote type script>
                vote_cell_hash_type: 0x01
                description: "Fund infrastructure work Q3 2026"
                receiver:
                    code_hash: <secp256k1 code hash>
                    hash_type: 0x01
                    args: <20-byte blake160 of receiver pubkey>
                amount: 1000
                minimal_requirement: 5000
        Type:
            code_hash: <proposal type script code_hash>
            hash_type: <proposal type script hash_type>
            args:
                <20-byte blake160 Type ID>
                <32-byte SP1 verifying key hash>
        Lock:
            code_hash: <always-success lock code_hash>
            hash_type: <always-success lock hash_type>
            args: <empty>

    <vec> Treasury_Cell
        Data: <empty>
        Type: <none>
        Lock: <described in treasure lock script spec>

Outputs:
    Receiver_Cell
        Data: <empty>
        Type: <none>
        Lock:
            code_hash: <secp256k1 code hash>
            hash_type: 0x01
            args: <20-byte blake160 of receiver pubkey> # must match Proposal.receiver
        Capacity: <Proposal.amount>

    Change_Cell
        Data: <empty>
        Type: <none>
        Lock:
            code_hash: <treasury lock script code_hash>
            hash_type: <treasury lock script hash_type>
            args: <empty>

Header Deps:
    header_deps[0]: <start block hash>                  # block containing the proposal cell
    header_deps[1]: <end block hash>                    # start block + duration blocks later

Witnesses:
    WitnessArgs structure (at index matching Proposal_Cell input):
        Lock: <empty>
        Input Type: <none>
        Output Type: ProposalWitness
            proof: <PLONK proof bytes>
            public_values (PublicValues molecule):
                proposal:
                    duration: 8640
                    vote_cell_code_hash: <32-byte hash of vote type script>
                    vote_cell_hash_type: 0x01
                    description: "Fund infrastructure work Q3 2026"
                    receiver:
                        code_hash: <secp256k1 code hash>
                        hash_type: 0x01
                        args: <20-byte blake160 of receiver pubkey>
                    amount: 1000
                    minimal_requirement: 5000
                start_block_hash: <32-byte hash>        # must equal header_deps[0]
                end_block_hash: <32-byte hash>          # must equal header_deps[1]
                proposal_script:
                    code_hash: <proposal type script code_hash>
                    hash_type: <proposal type script hash_type>
                    args: <20-byte blake160 Type ID> <32-byte SP1 verifying key hash>
                passed: 0x01                            # 1 = proposal passed
```
