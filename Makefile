

sp1-run:
	cd sp1/ckb-vote-verification/script && cargo run --release -- --execute

sp1-profiling:
	cd sp1/ckb-vote-verification/script && cargo run --release --features profiling -- --execute

fmt:
	cargo fmt
	cd sp1/ckb-vote-verification && cargo fmt

prove-via-network:
	cd sp1/ckb-vote-verification/script && RUST_LOG=info cargo run --release --bin prove-via-network
