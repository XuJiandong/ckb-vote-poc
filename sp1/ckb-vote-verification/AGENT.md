
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
