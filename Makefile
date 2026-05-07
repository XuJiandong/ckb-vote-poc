

sp1-run:
	cd sp1/ckb-vote-verification/script && cargo run --release -- --execute

fmt:
	cargo fmt
	cd sp1/ckb-vote-verification && cargo fmt
