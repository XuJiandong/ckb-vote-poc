#![no_main]
sp1_zkvm::entrypoint!(main);

use ckb_vote_types::molecules::types::BlockVec;
use molecule::prelude::Entity;

pub fn main() {
    let block_data = sp1_zkvm::io::read_vec();

    let blocks = BlockVec::from_slice(&block_data).expect("failed to parse block data");

    ckb_vote_verification::verify_block_integrity(&blocks)
        .expect("block integrity verification failed");

    let first_block = blocks.get(0).expect("should have at least one block");
    let start_hash = ckb_vote_verification::compute_header_hash(&first_block.header());

    let last_idx = blocks.len().saturating_sub(1);
    let last_block = blocks.get(last_idx).expect("should exist");
    let end_hash = ckb_vote_verification::compute_header_hash(&last_block.header());

    sp1_zkvm::io::commit(&start_hash);
    sp1_zkvm::io::commit(&end_hash);
}
