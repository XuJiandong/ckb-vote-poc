use ckb_vote_types::molecules::types::BlockVecReader;
use ckb_vote_types::molecules::verify_block_vec;
use molecule::prelude::Reader;

const BLOCK_DATA: &[u8] = include_bytes!("blocks.bin");

#[test]
fn test_block_integrity() {
    verify_block_vec(BLOCK_DATA, false).expect("verify_block_vec");
    let args = ckb_vote_verification::prepare_guest_program_arguments(BLOCK_DATA);
    let args_reader = args.as_reader();
    let blocks = BlockVecReader::new_unchecked(args_reader.blocks().raw_data());
    ckb_vote_verification::verify_block_integrity(blocks, args_reader.witness_root())
        .expect("block integrity check");
}
