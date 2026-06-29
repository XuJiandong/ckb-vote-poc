# Native Benchmark: `count_vote`

The benchmark exercises `ckb_vote_verification::count_vote` over a synthetic
voting window built from 500 real CKB mainnet blocks (sourced from
`crates/verification/tests/blocks.bin`).

Run the benchmark yourself:

```sh
cargo bench --bench count_vote -p ckb-vote-verification
```

---

## Machine 1 — Apple M4 (macOS)

### Environment

| Field  | Value                      |
|--------|----------------------------|
| OS     | macOS 26.5.1 (Build 25F80) |
| CPU    | Apple M4                   |

### Result

| Metric | Time      |
|--------|-----------|
| Mean   | 177.70 µs |
| Low    | 176.71 µs |
| High   | 179.07 µs |

### Estimate: cost per day at 10 s/block

CKB targets one block every ~10 seconds.

| Parameter                     | Value                                 |
|-------------------------------|---------------------------------------|
| Block interval                | 10 s                                  |
| Blocks per day                | 86,400 s ÷ 10 s = **8,640 blocks**    |
| Benchmark sample              | 500 blocks → 177.70 µs                |
| Cost per block (linear scale) | 177.70 µs ÷ 500 ≈ **0.355 µs/block** |
| Estimated cost per day        | 8,640 × 0.355 µs ≈ **3.1 ms**        |

Processing an entire day's worth of blocks takes roughly **3.1 ms** of CPU time.

---

## Machine 2 — Intel Xeon E5-2670 v2 (Linux) ⚠️ Low-end server

> **Note:** This is a very low-end machine — an aging dual-socket Xeon from
> 2013 running at a reduced clock of 1.2 GHz (thermal/power throttling). Results
> here represent a conservative lower bound.

### Environment

| Field   | Value                                        |
|---------|----------------------------------------------|
| OS      | Linux 6.12.90+deb13.1-amd64 (Debian 13)     |
| CPU     | Intel Xeon E5-2670 v2 @ 2.50 GHz (10C/20T)  |

### Result

| Metric | Time      |
|--------|-----------|
| Mean   | 482.95 µs |
| Low    | 482.70 µs |
| High   | 483.33 µs |

### Estimate: cost per day at 10 s/block

| Parameter                     | Value                                 |
|-------------------------------|---------------------------------------|
| Block interval                | 10 s                                  |
| Blocks per day                | 86,400 s ÷ 10 s = **8,640 blocks**    |
| Benchmark sample              | 500 blocks → 482.95 µs                |
| Cost per block (linear scale) | 482.95 µs ÷ 500 ≈ **0.966 µs/block** |
| Estimated cost per day        | 8,640 × 0.966 µs ≈ **8.3 ms**        |

Even on this low-end machine, processing an entire day's worth of blocks takes
roughly **8.3 ms** of CPU time — still well under any practical budget.
