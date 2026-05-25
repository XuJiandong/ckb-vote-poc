#![no_main]
sp1_zkvm::entrypoint!(main);

use ckb_vote_types::molecules::{
    blockchain,
    types::{BlockVecReader, GuestProgramArgumentsReader, PublicValues},
};
use molecule::prelude::{Builder, Entity, Reader};

pub fn main() {
    let args_bytes = sp1_zkvm::io::read_vec();
    let args = GuestProgramArgumentsReader::from_slice(&args_bytes)
        .expect("failed to load guest program arguments");

    let blocks_bytes = args.blocks().raw_data();
    ckb_vote_types::molecules::verify_block_vec(blocks_bytes, false)
        .expect("failed to verify BlockVec in molecule format");
    let blocks = BlockVecReader::new_unchecked(blocks_bytes);
    let witness_root = args.witness_root();

    ckb_vote_verification::verify_block_integrity(blocks, witness_root)
        .expect("block integrity verification failed");

    let first_block = blocks.get(0).expect("should have at least one block");
    let start_hash = ckb_vote_verification::compute_header_hash(first_block.header());

    let last_idx = blocks.len().saturating_sub(1);
    let last_block = blocks.get(last_idx).expect("should exist");
    let end_hash = ckb_vote_verification::compute_header_hash(last_block.header());

    let result = ckb_vote_verification::count_vote(blocks, args.proposal_script().to_entity());

    let public_values = PublicValues::new_builder()
        .proposal(result.proposal)
        .start_block_hash(blockchain::Byte32::from(start_hash))
        .end_block_hash(blockchain::Byte32::from(end_hash))
        .proposal_script(args.proposal_script().to_entity())
        .passed(blockchain::Byte::from(if result.passed {
            1u8
        } else {
            0u8
        }))
        .yes_vote(blockchain::Uint64::from(result.yes_vote.to_le_bytes()))
        .no_vote(blockchain::Uint64::from(result.no_vote.to_le_bytes()))
        .build();

    sp1_zkvm::io::commit_slice(public_values.as_slice());
}
