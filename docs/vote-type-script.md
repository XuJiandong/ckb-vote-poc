# Vote Type Script Specification
The vote type script is used together with the [proposal type script](./proposal-type-script.md). The `vote_cell_code_hash` and `vote_cell_hash_type` fields in that spec identify this vote type script.

## Script

```
code_hash: <vote type script code hash>
hash_type: <vote type script hash type>
args: <20 bytes blake160 hash of proposal type script>
```
The `args` field holds a blake160 hash of the proposal type script that this vote is cast for.
Since a proposal type script is unique across the entire chain, there is no ambiguity.


## Cell Data
One byte data: 0 for "NO" and 1 for "YES".

## Witness
This script doesn't read witness.

## Unlocking process

When a vote cell is created, the action is treated as casting a vote. The script first checks that a proposal cell — identified by the blake160 hash stored in `args` — exists in `cell_deps`. It is not required to validate the proposal cell in full; it only checks that the proposal cell exists. The script then looks for a lock script on the input cells that matches the corresponding output lock script; this lock script represents ownership of the DAO. The script then traverses all `cell_deps` to find cells meeting both of the following conditions:

1. Its lock script matches the DAO owner.
2. Its type script is the Nervos DAO, as described [here](https://github.com/nervosnetwork/rfcs/blob/master/rfcs/0024-ckb-genesis-script-list/0024-ckb-genesis-script-list.md#nervos-dao).

If no such `cell_dep` exists, the script fails. Tallying the total CKB voted is the guest program's responsibility — the vote type script does not perform this count.


When a vote cell is consumed, there is no special meaning — it simply recycles the occupied CKB. The cell can be consumed at any time and does not need to wait until voting ends.

## Design Notes

* A "NO" vote is generally unnecessary — users can simply do nothing. However, it can be used to retract a previous "YES" vote: later votes from the same voter overwrite earlier ones, so casting a "NO" after a "YES" effectively cancels that prior vote. This is enforced in the guest program, not by this script.

* Once a proposal cell is consumed, it can no longer be referenced in `cell_deps`, which prevents new votes from being cast after the proposal closes.

## Examples

### Example 1: Creating a Vote Cell (Casting a YES Vote)

```yaml
Cell Deps:
    Proposal_Cell                                       # the proposal this vote is cast for
        Data: <proposal cell data>
        Type:
            code_hash: <proposal type script code hash>
            hash_type: <proposal type script hash type>
            args:
                <20-byte blake160 of prev TX> <32-byte SP1 verifying key>
        Lock:
            <always-success lock script>

    DAO_Deposit_Cell                                    # voter's DAO deposit, referenced to prove vote weight
        Data: <DAO deposit data>
        Type:
            code_hash: <Nervos DAO code hash>           # genesis DAO type script
            hash_type: 0x01                             # type
            args: <empty>
        Lock:
            <voter's lock script>                       # must match the lock on Vote_Cell output

Inputs:
    Funding_Cell
        Data: <empty>
        Type: <none>
        Lock:
            <voter's lock script>                       # same lock as DAO_Deposit_Cell and Vote_Cell output

Outputs:
    Vote_Cell
        Data: 0x01                                      # 1 = YES
        Type:
            code_hash: <vote type script code hash>
            hash_type: <vote type script hash type>
            args:
                <20-byte blake160 of proposal type script>  # identifies which proposal this vote is for
        Lock:
            <voter's lock script>

    <any> Change_Cell
        Data: <empty>
        Type: <none>
        Lock:
            <voter's lock script>

Witnesses:
    WitnessArgs structure (at index matching Funding_Cell input):
        Lock: <voter's signature>
        Input Type: <none>
        Output Type: <none>                             # vote type script does not read witness
```

---

### Example 2: Consuming a Vote Cell (Recycling CKB)

```yaml
Inputs:
    Vote_Cell                                           # the previously cast vote
        Data: 0x01                                      # 1 = YES (original vote content)
        Type:
            code_hash: <vote type script code hash>
            hash_type: <vote type script hash type>
            args:
                <20-byte blake160 of proposal type script>
        Lock:
            <voter's lock script>

Outputs:
    <any> Change_Cell
        Data: <empty>
        Type: <none>
        Lock:
            <voter's lock script>                       # voter reclaims the occupied CKB

Witnesses:
    WitnessArgs structure (at index matching Vote_Cell input):
        Lock: <voter's signature>
        Input Type: <none>
        Output Type: <none>                             # vote type script does not read witness
```
