# Test Data

`blocks.bin` contains 5 consecutive CKB mainnet blocks in `BlockVec` molecule format.

## Source

Fetched using the `block-dumper` tool:

```sh
cargo run -p block-dumper -- \
  --start-block-hash 0x1c3e9243aa93404c5f34766b3ad9d921a6e1c7ae8c5a091b09ad5a0c140e8cd5 \
  --count 500 \
  --out crates/verification/tests/blocks.bin
```

| Parameter | Value |
|-----------|-------|
| RPC endpoint | `https://mainnet.ckb.dev` (default) |
| Start block hash | `0x1c3e9243aa93404c5f34766b3ad9d921a6e1c7ae8c5a091b09ad5a0c140e8cd5` |
| Count | 5 |

The first block is fetched by hash via `get_block`, and the remaining 4 are
fetched by number via `get_block_by_number`. All blocks are requested with
verbosity `0x0` (hex-encoded molecule format), then concatenated into a
`BlockVec` and written to disk.
