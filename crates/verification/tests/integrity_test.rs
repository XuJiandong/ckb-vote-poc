use ckb_vote_types::molecules::blockchain;
use ckb_vote_types::molecules::types::BlockVecReader;
use ckb_vote_types::molecules::verify_block_vec;
use ckb_vote_verification::Error;
use molecule::prelude::{Builder, Entity, Reader};

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

#[test]
fn test_witness_root_length_mismatch_returns_error() {
    verify_block_vec(BLOCK_DATA, false).expect("verify_block_vec");
    let args = ckb_vote_verification::prepare_guest_program_arguments(BLOCK_DATA);
    let args_reader = args.as_reader();
    let blocks = BlockVecReader::new_unchecked(args_reader.blocks().raw_data());
    let witness_root = blockchain::Byte32Vec::new_builder().build();

    let err = ckb_vote_verification::verify_block_integrity(blocks, witness_root.as_reader())
        .expect_err("witness_root length mismatch should fail");

    assert_eq!(
        err,
        Error::WitnessRootLengthMismatch {
            expected: blocks.len(),
            actual: 0
        }
    );
}

#[test]
fn test_extra_witness_root_returns_error() {
    verify_block_vec(BLOCK_DATA, false).expect("verify_block_vec");
    let args = ckb_vote_verification::prepare_guest_program_arguments(BLOCK_DATA);
    let args_reader = args.as_reader();
    let blocks = BlockVecReader::new_unchecked(args_reader.blocks().raw_data());
    let roots = args_reader
        .witness_root()
        .iter()
        .map(|root| root.to_entity())
        .chain(std::iter::once([0u8; 32].into()))
        .collect();
    let witness_root = blockchain::Byte32Vec::new_builder().set(roots).build();

    let err = ckb_vote_verification::verify_block_integrity(blocks, witness_root.as_reader())
        .expect_err("extra witness_root should fail");

    assert_eq!(
        err,
        Error::WitnessRootLengthMismatch {
            expected: blocks.len(),
            actual: blocks.len() + 1
        }
    );
}
