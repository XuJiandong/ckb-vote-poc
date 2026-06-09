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
- **SP1 Guest Program**: The `sp1/ckb-vote-verification/program` is compiled targeting RISC-V for the zkVM, using a custom toolchain provided by SP1. Do not mix this environment or its build artifacts with other Rust projects. The sp1up version `cargo-prove sp1 (d454975 2026-04-11T01:51:47.829463000Z)`.
- **on-chain scripts**: The projects in `contracts` are compiled targeting RISC-V for CKB, using stable Rust 1.92.0. Do not mix this environment or its build artifacts with other Rust projects.

## Documents

The design document is at `docs/design/README.md`. The `docs/*.md` files contain specifications.

- When using the CCC library, refer to `docs/knowledge/ccc.md`.
- When using the `ckb-cli` tool, refer to `docs/knowledge/ckb-cli.md`.
- When working with the devnet, refer to `docs/knowledge/devnet.md`.
- When working with CKB RPC, refer to `docs/knowledge/rpc.md`.

## Code generation

`crates/types/build.rs` generates Rust code from `.mol` schema files (in `crates/types/molecules/`) using `molecule-codegen`. The generated code lives in `crates/types/src/molecules/`.

`sp1/ckb-vote-verification/script/build.rs` builds the zkVM guest program via `sp1-build`.

## Profile Guest Program
When profiling sp1/ckb-vote-verifier/program, See [docs/guest-program-profiling.md](docs/guest-program-profiling.md).

## Development workflow

After making changes, run the following in order:

### 1. Format

```sh
make fmt
```

### 2. Test

```sh
make test
```

### 3. Build and execute (SP1)

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
