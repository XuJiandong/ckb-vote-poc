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
- `cargo fmt` uses edition 2024 formatting in both workspaces

## Code generation

`crates/types/build.rs` generates Rust code from `.mol` schema files (in `crates/types/molecules/`) using `molecule-codegen`. The generated code lives in `crates/types/src/molecules/`.

`sp1/ckb-vote-verification/script/build.rs` builds the zkVM guest program via `sp1-build`.

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

## Key dependencies

- SP1 SDK: 6.1.0
- molecule: 0.9.2 (for CKB data structure serialization)
- ckb-gen-types: 1.1.0
- ckb-hash: 1.1.0
