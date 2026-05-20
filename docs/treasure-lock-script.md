# Treasury Lock Script Specification

The treasury lock script is used together with the [proposal type script](./proposal-type-script.md).

## Script

```
code_hash: <treasury lock script code_hash>
hash_type: <treasury lock script hash_type>
args: <empty>
```

This script is unlocked when a proposal cell is consumed, so no `args` are needed. Anyone can submit the SP1 proof and construct the unlocking transaction — there is no restriction on who triggers it.

## Unlocking Process

The script has following hard-coded or configured parameters:

- proposal type script `code_hash` / `hash_type`
- verifying key of the guest program

These parameters are critical to the voting system. The verifying key is tied to the guest program binary — it must be updated whenever the guest program changes. Allowing these values to be malformed or overwritten would be a serious security issue.

The script follows these steps:

1. Verify there is exactly one input cell whose type script matches the configured proposal type script `code_hash` / `hash_type`.
2. The trailing 32 bytes of `args` of proposal type script should match verifying key
3. Verify the corresponding cell data conforms to the [Proposal](./proposal-type-script.md#cell-data) structure.
4. Verify there is an output cell carrying exactly `amount` CKB locked to the `receiver` script — this is the funding disbursed when a proposal passes. There must be exactly two output cells: one for the receiver and one change cell locked with the treasury lock script. The total input capacity and total output capacity must differ by no more than a small transaction fee (e.g. < 1 CKB).
5. Re-validate the `duration`, `vote_cell_code_hash` / `vote_cell_hash_type`, `amount`, and `minimal_requirement` in `proposal` field in `PublicValues` against the cell data, even though the proposal type script already checks it.

## Examples

### Example 1: Unlocking Treasury Cells When a Proposal Passes

```yaml
Inputs:
    Proposal_Cell                                       # exactly one; type script must match configured code_hash/hash_type
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
            code_hash: <proposal type script code_hash>     # must match configured value
            hash_type: <proposal type script hash_type>     # must match configured value
            args:
                <20-byte blake160 Type ID>
                <32-byte SP1 verifying key hash>            # trailing 32 bytes must match configured verifying key
        Lock:
            code_hash: <always-success lock code_hash>
            hash_type: <always-success lock hash_type>
            args: <empty>

    <vec> Treasury_Cell                                 # one or more; each unlocked by this script
        Data: <empty>
        Type: <none>
        Lock:
            code_hash: <treasury lock script code_hash>
            hash_type: <treasury lock script hash_type>
            args: <empty>

Outputs:
    Receiver_Cell                                       # exactly one; carries exactly Proposal.amount CKB
        Data: <empty>
        Type: <none>
        Lock:
            code_hash: <secp256k1 code hash>
            hash_type: 0x01
            args: <20-byte blake160 of receiver pubkey>     # must match Proposal.receiver
        Capacity: <Proposal.amount>
    Change_Cell                                         # exactly one; locked with treasury lock script
        Data: <empty>
        Type: <none>
        Lock:
            code_hash: <treasury lock script code_hash>
            hash_type: <treasury lock script hash_type>
            args: <empty>
        Capacity: <total_input_capacity - Proposal.amount - tx_fee>
        # tx_fee must be < 1 CKB

Header Deps:
    header_deps[0]: <start block hash>                  # block containing the proposal cell
    header_deps[1]: <end block hash>                    # start block + duration blocks later

Witnesses:
    WitnessArgs structure (at index matching Proposal_Cell input):
        Lock: <empty>
        Input Type: <none>
        Output Type: SP1ProofWithPublicValues
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
