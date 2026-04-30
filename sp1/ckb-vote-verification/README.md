
## SP1 version 
version 6.1.0

```
❯ cargo prove --version
cargo-prove sp1 (d454975 2026-04-11T01:51:47.829463000Z)
```

## Building the Guest Program

To build the guest program, run:
```sh
cd program && cargo prove build
```

## Running the Guest Program

To execute the guest program natively:
```sh
cd script && RUST_LOG=info cargo run --release -- --execute
```

## Generating a Proof

To generate a proof for the guest program:
```sh
cd script && RUST_LOG=info cargo run --release -- --prove
```

**Note:** Proof generation may take significant time and use substantial CPU resources.
