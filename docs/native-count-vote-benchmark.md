# Native Benchmark: `count_vote`

## Environment

| Field    | Value                   |
|----------|-------------------------|
| OS       | macOS 26.5.1 (Build 25F80) |
| CPU      | Apple M4                |
| Memory   | 16 GB                   |

## Result

The benchmark exercises `ckb_vote_verification::count_vote` over a synthetic
voting window built from 500 real CKB mainnet blocks (sourced from
`crates/verification/tests/blocks.bin`).

| Metric | Time       |
|--------|------------|
| Mean   | 177.70 µs  |
| Low    | 176.71 µs  |
| High   | 179.07 µs  |

Run the benchmark yourself:

```sh
cargo bench --bench count_vote -p ckb-vote-verification
```

## Estimate: cost per day at 10 s/block

CKB targets one block every ~10 seconds.

| Parameter                     | Value                                  |
|-------------------------------|----------------------------------------|
| Block interval                | 10 s                                   |
| Blocks per day                | 86,400 s ÷ 10 s = **8,640 blocks**     |
| Benchmark sample              | 500 blocks → 177.70 µs                 |
| Cost per block (linear scale) | 177.70 µs ÷ 500 ≈ **0.355 µs/block**  |
| Estimated cost per day        | 8,640 × 0.355 µs ≈ **3.1 ms**         |

In other words, processing an entire day's worth of blocks through `count_vote`
takes roughly **3 ms** of CPU time on Apple M4 — well under any practical budget.
