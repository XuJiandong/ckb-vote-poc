#![no_main]
sp1_zkvm::entrypoint!(main);

use ckb_vote_types::molecules::types::{BlockVecReader, GuestProgramArgumentsReader};
use ckb_vote_types::molecules::verify_block_vec;
use molecule::prelude::Reader;

pub fn main() {
    let args_bytes = sp1_zkvm::io::read_vec();
    let args = GuestProgramArgumentsReader::from_slice(&args_bytes)
        .expect("failed to load guest program arguments");

    let blocks_bytes = args.blocks().raw_data();
    verify_block_vec(blocks_bytes, false).expect("failed to verify BlockVec in molecule format");
    let blocks = BlockVecReader::new_unchecked(blocks_bytes);
    let witness_root = args.witness_root();

    ckb_vote_verification::verify_block_integrity(blocks, witness_root)
        .expect("block integrity verification failed");

    let first_block = blocks.get(0).expect("should have at least one block");
    let start_hash = ckb_vote_verification::compute_header_hash(first_block.header());

    let last_idx = blocks.len().saturating_sub(1);
    let last_block = blocks.get(last_idx).expect("should exist");
    let end_hash = ckb_vote_verification::compute_header_hash(last_block.header());

    let stats = ckb_vote_verification::count_vote(blocks, args.proposal_script().to_entity());

    sp1_zkvm::io::commit(&start_hash);
    sp1_zkvm::io::commit(&end_hash);
    sp1_zkvm::io::commit(&stats.block_count);
    sp1_zkvm::io::commit(&stats.transaction_count);
    sp1_zkvm::io::commit(&stats.lock_scripts);
    sp1_zkvm::io::commit(&stats.type_scripts);
    sp1_zkvm::io::commit(&stats.cell_deps);
}
