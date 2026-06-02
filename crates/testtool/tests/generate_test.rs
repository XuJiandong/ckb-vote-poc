use ckb_vote_types::molecules::{
    blockchain,
    types::{BlockVecReader, Proposal},
};
use molecule::prelude::{Builder, Entity, Reader};

// blocks.bin contains 500 consecutive CKB mainnet blocks.
const BLOCK_DATA: &[u8] = include_bytes!("../../verification/tests/blocks.bin");

#[test]
fn test_generate_and_count_vote() {
    // duration is not set here; generate_from_templates will stamp it as num_blocks - 1.
    let proposal = Proposal::new_builder()
        .vote_cell_code_hash(blockchain::Byte32::from([1u8; 32]))
        .vote_cell_hash_type(blockchain::Byte::new(0))
        .minimal_requirement(blockchain::Uint64::from(100u64.to_le_bytes()))
        .build();

    let args = ckb_vote_testtool::generate_from_templates(proposal, BLOCK_DATA)
        .expect("generate_from_templates");

    let args_reader = args.as_reader();
    let blocks = BlockVecReader::new_unchecked(args_reader.blocks().raw_data());
    let num_blocks = blocks.len();
    let proposal_script = args_reader.proposal_script().to_entity();

    // Block 0 gets the proposal tx; each of the remaining (num_blocks - 1) blocks gets one
    // YES vote of 100 shannons with a unique voter lock, so all votes accumulate.
    let result = ckb_vote_verification::count_vote(blocks, proposal_script);
    assert_eq!(result.yes_vote, (num_blocks as u64 - 1) * 100);
    assert_eq!(result.no_vote, 0);
    assert!(result.passed);
}
