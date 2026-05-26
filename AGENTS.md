# CKB Vote PoC — Agent Instructions

## Project overview

CKB vote verification system using SP1 zkVM. Two Cargo workspaces:

| Workspace | Location | Edition | Resolver | Members |
|-----------|----------|---------|----------|---------|
| Root | `./Cargo.toml` | 2024 | 3 | `crates/types`, `crates/verification`, `tools/block-dumper` |
| SP1 | `sp1/ckb-vote-verification/Cargo.toml` | 2024 | 3 | `program`, `script` |

The SP1 guest program (`program`) depends on `crates/verification` and `crates/types` via path dependency.

## Toolchains

- **Root**: `rust-toolchain.toml` pins Rust 1.92.0
- **SP1**: `sp1/ckb-vote-verification/rust-toolchain` uses `stable` with `llvm-tools` + `rustc-dev`
- **SP1 Guest Program**: The `sp1/ckb-vote-verification/program` is compiled targeting RISC-V for the zkVM, using a custom toolchain provided by SP1. Do not mix this environment or its build artifacts with other Rust projects.
- `cargo fmt` uses edition 2024 formatting in both workspaces

## Documents

The design document is at `docs/design/README.md`. The `docs/*.md` files contain specifications.

## Code generation

`crates/types/build.rs` generates Rust code from `.mol` schema files (in `crates/types/molecules/`) using `molecule-codegen`. The generated code lives in `crates/types/src/molecules/`.

`sp1/ckb-vote-verification/script/build.rs` builds the zkVM guest program via `sp1-build`.

## Profile Guest Program

Profile `sp1/ckb-vote-verification/program` to see where zkVM cycles are spent.

### Setup

The `script/Cargo.toml` enables the `profiling` feature on `sp1-sdk`:

```toml
sp1-sdk = { workspace = true, features = ["profiling"] }
```

### Generate a trace

```sh
# Build the guest program
cd sp1/ckb-vote-verification/program && cargo prove build

# Execute with profiling enabled. TRACE_SAMPLE_RATE controls sampling
# (1 in N cycles); use for larger programs to keep the trace file small.
cd sp1/ckb-vote-verification/script \
  && TRACE_FILE=trace.json TRACE_SAMPLE_RATE=100 RUST_LOG=info cargo run --release -- --execute
```

### Visualize

Install [samply](https://github.com/mstange/samply) (`cargo install --locked samply`), then:

```sh
cd sp1/ckb-vote-verification/script && samply load trace.json
```

Open the Firefox Profiler URL it prints. The "time" axis in the profiler is actually cycle count — fewer cycles in a call frame is better.

### Cycle Tracking
Use [this](https://docs.succinct.xyz/docs/sp1/optimizing-programs/cycle-tracking)

## Development workflow

After making changes, run the following in order:

### 1. Format

```sh
# Root workspace
cargo fmt

# SP1 workspace
cd sp1/ckb-vote-verification && cargo fmt
```

### 2. Test

```sh
cargo test
```

Tests are in `crates/verification/tests/` (integration test using `blocks.bin` fixture).

### 3. Build and execute (SP1)

```sh
# Build the zkVM guest program
cd sp1/ckb-vote-verification/program && cargo prove build

# Execute (native, without proof generation)
cd sp1/ckb-vote-verification/script && RUST_LOG=info cargo run --release -- --execute
```

or simply

```sh
make sp1-run
```

## On-Chain Script (Contract) Implementation

These scripts should be implemented in Rust using [ckb-std](https://github.com/nervosnetwork/ckb-std).
When using syscalls, prefer the `high_level` API. If a high-level equivalent is unavailable, fall back to the low-level syscalls.
Review the relevant [RFCs](https://github.com/nervosnetwork/rfcs/tree/master/rfcs) before starting implementation.

## Key dependencies

- SP1 SDK: 6.1.0
- molecule: 0.9.2 (for CKB data structure serialization)
- ckb-gen-types: 1.1.0
- ckb-hash: 1.1.0
