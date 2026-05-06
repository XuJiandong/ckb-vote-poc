GENERATED_DIR := src/molecules

generate:
	@set -eu; \
	moleculec --language rust --schema-file molecules/types.mol > $(GENERATED_DIR)/types.rs; \
	cargo fmt -- $(GENERATED_DIR)/types.rs
