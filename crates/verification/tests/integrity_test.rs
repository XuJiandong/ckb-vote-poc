use ckb_vote_types::molecules::types::BlockVec;
use molecule::prelude::Entity;

const BLOCK_DATA: &[u8] = include_bytes!("blocks.bin");

#[test]
fn test_block_integrity() {
    let block_vec = BlockVec::from_slice(BLOCK_DATA).expect("deserialize BlockVec");
    ckb_vote_verification::verify_block_integrity(&block_vec).expect("block integrity check");
}
