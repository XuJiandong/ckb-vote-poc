# Profile Guest Program

Profile `sp1/ckb-vote-verification/program` to see where zkVM cycles are spent.

## Setup

The `script/Cargo.toml` enables the `profiling` feature on `sp1-sdk`:

```toml
sp1-sdk = { workspace = true, features = ["profiling"] }
```

## Generate a trace

```sh
# Build the guest program
cd sp1/ckb-vote-verification/program && cargo prove build

# Execute with profiling enabled. TRACE_SAMPLE_RATE controls sampling
# (1 in N cycles); use for larger programs to keep the trace file small.
cd sp1/ckb-vote-verification/script \
  && TRACE_FILE=trace.json TRACE_SAMPLE_RATE=100 RUST_LOG=info cargo run --release -- --execute
```

## Visualize

Install [samply](https://github.com/mstange/samply) (`cargo install --locked samply`), then:

```sh
cd sp1/ckb-vote-verification/script && samply load trace.json
```

Open the Firefox Profiler URL it prints. The "time" axis in the profiler is actually cycle count — fewer cycles in a call frame is better.

## Cycle Tracking

Use [this](https://docs.succinct.xyz/docs/sp1/optimizing-programs/cycle-tracking).
