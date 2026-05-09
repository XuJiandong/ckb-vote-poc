use ckb_vote_types::molecules::types::BlockVec;
use ckb_vote_types::molecules::verify_block_vec;

use molecule::prelude::Entity;

const BLOCK_DATA: &[u8] = include_bytes!("blocks.bin");

#[test]
fn test_block_integrity() {
    verify_block_vec(&BLOCK_DATA, false).expect("verify_block_vec");
    let block_vec = BlockVec::new_unchecked(BLOCK_DATA.into());
    ckb_vote_verification::verify_block_integrity(&block_vec).expect("block integrity check");
}
