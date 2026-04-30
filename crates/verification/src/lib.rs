use blake2::Blake2bVarCore;
use blake2::digest::Update;
use blake2::digest::core_api::{CoreWrapper, VariableOutputCore};
use blake2::digest::generic_array::GenericArray;
use blake2::digest::typenum::U64;
use ckb_vote_types::molecules::{blockchain, types::BlockVec};
use molecule::prelude::{Entity, Reader};

const CKB_HASH_PERSONALIZATION: &[u8] = b"ckb-default-hash";

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    EmptyBlocks,
    ParentHashMismatch { block_index: usize },
    TransactionsRootMismatch { block_index: usize },
}

fn blake2b_256(data: &[u8]) -> [u8; 32] {
    let core = Blake2bVarCore::new_with_params(&[], CKB_HASH_PERSONALIZATION, 0, 32);
    let mut wrapper = CoreWrapper::<Blake2bVarCore>::from_core(core);
    Update::update(&mut wrapper, data);
    let mut full_res: GenericArray<u8, U64> = Default::default();
    let (mut core, mut buffer) = wrapper.decompose();
    core.finalize_variable_core(&mut buffer, &mut full_res);
    let mut result = [0u8; 32];
    result.copy_from_slice(&full_res[..32]);
    result
}

fn merkle_root(leaves: &[[u8; 32]]) -> [u8; 32] {
    if leaves.is_empty() {
        return [0u8; 32];
    }
    let mut nodes = leaves.to_vec();
    while nodes.len() > 1 {
        if nodes.len() % 2 == 1 {
            nodes.push(nodes[nodes.len() - 1]);
        }
        let mut next = Vec::with_capacity(nodes.len() / 2);
        for i in (0..nodes.len()).step_by(2) {
            let mut data = [0u8; 64];
            data[..32].copy_from_slice(&nodes[i]);
            data[32..].copy_from_slice(&nodes[i + 1]);
            next.push(blake2b_256(&data));
        }
        nodes = next;
    }
    nodes[0]
}

fn byte32_to_arr(b: &blockchain::Byte32) -> [u8; 32] {
    let mut arr = [0u8; 32];
    arr.copy_from_slice(b.as_slice());
    arr
}

fn header_hash(header: &blockchain::Header) -> [u8; 32] {
    blake2b_256(header.as_slice())
}

fn tx_hash(tx: &blockchain::TransactionReader<'_>) -> [u8; 32] {
    blake2b_256(tx.raw().as_slice())
}

fn witness_hash(tx: &blockchain::TransactionReader<'_>) -> [u8; 32] {
    blake2b_256(tx.as_slice())
}

fn calc_transactions_root(block: &blockchain::Block) -> [u8; 32] {
    let reader = block.as_reader();
    let txs = reader.transactions();
    let tx_hashes: Vec<[u8; 32]> = txs.iter().map(|tx| tx_hash(&tx)).collect();
    let witness_hashes: Vec<[u8; 32]> = txs.iter().map(|tx| witness_hash(&tx)).collect();
    let raw_root = merkle_root(&tx_hashes);
    let witness_root = merkle_root(&witness_hashes);
    merkle_root(&[raw_root, witness_root])
}

pub fn compute_header_hash(header: &blockchain::Header) -> [u8; 32] {
    blake2b_256(header.as_slice())
}

pub fn verify_block_integrity(blocks: &BlockVec) -> Result<(), Error> {
    if blocks.is_empty() {
        return Err(Error::EmptyBlocks);
    }

    for i in 1..blocks.len() {
        let prev_block = blocks.get(i - 1).expect("should exist");
        let current_block = blocks.get(i).expect("should exist");
        let prev_hash = header_hash(&prev_block.header());
        let parent_hash = byte32_to_arr(&current_block.header().raw().parent_hash());
        if prev_hash != parent_hash {
            return Err(Error::ParentHashMismatch { block_index: i });
        }
    }

    for i in 0..blocks.len() {
        let block = blocks.get(i).expect("should exist");
        let expected_root = byte32_to_arr(&block.header().raw().transactions_root());
        let actual_root = calc_transactions_root(&block);
        if expected_root != actual_root {
            return Err(Error::TransactionsRootMismatch { block_index: i });
        }
    }

    Ok(())
}
