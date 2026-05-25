use blake2::{Blake2b256, Digest, digest::CustomizedInit};
use ckb_vote_types::molecules::{
    blockchain::{self, Script},
    types::{BlockVec, BlockVecReader, GuestProgramArguments, Proposal, Vote},
};
use molecule::prelude::{Builder, Entity, Reader};
use std::collections::{BTreeMap, VecDeque};

const CKB_HASH_PERSONALIZATION: &[u8] = b"ckb-default-hash";

#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    EmptyBlocks,
    ParentHashMismatch { block_index: usize },
    TransactionsRootMismatch { block_index: usize },
}

fn blake2b_256(data: &[u8]) -> [u8; 32] {
    #[cfg(feature = "profiling")]
    println!("cycle-tracker-report-start: blake2b");
    let mut hasher = Blake2b256::new_customized(CKB_HASH_PERSONALIZATION);
    hasher.update(data);
    let result = hasher.finalize().into();
    #[cfg(feature = "profiling")]
    println!("cycle-tracker-report-end: blake2b");
    result
}

fn merge(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut data = [0u8; 64];
    data[..32].copy_from_slice(left);
    data[32..].copy_from_slice(right);
    blake2b_256(&data)
}

// Complete Binary Merkle Tree (CBMT), matching the merkle_cbt crate used by CKB.
pub fn merkle_root(leaves: &[[u8; 32]]) -> [u8; 32] {
    if leaves.is_empty() {
        return [0u8; 32];
    }

    let mut queue: VecDeque<[u8; 32]> = VecDeque::with_capacity((leaves.len() + 1) >> 1);

    let mut iter = leaves.rchunks_exact(2);
    while let Some([leaf1, leaf2]) = iter.next() {
        queue.push_back(merge(leaf1, leaf2));
    }
    if let [leaf] = iter.remainder() {
        queue.push_front(*leaf);
    }

    while queue.len() > 1 {
        let right = queue.pop_front().unwrap();
        let left = queue.pop_front().unwrap();
        queue.push_back(merge(&left, &right));
    }

    queue.pop_front().unwrap()
}

fn byte32_to_arr(b: blockchain::Byte32Reader<'_>) -> [u8; 32] {
    let mut arr = [0u8; 32];
    arr.copy_from_slice(b.as_slice());
    arr
}

fn header_hash(header: blockchain::HeaderReader<'_>) -> [u8; 32] {
    blake2b_256(header.as_slice())
}

fn tx_hash(tx: &blockchain::TransactionReader<'_>) -> [u8; 32] {
    blake2b_256(tx.raw().as_slice())
}

pub fn witness_hash(tx: &blockchain::TransactionReader<'_>) -> [u8; 32] {
    blake2b_256(tx.as_slice())
}

fn calc_transactions_root(
    block: blockchain::BlockReader<'_>,
    witness_root: blockchain::Byte32Reader<'_>,
) -> [u8; 32] {
    let txs = block.transactions();
    let tx_hashes: Vec<[u8; 32]> = txs.iter().map(|tx| tx_hash(&tx)).collect();
    let raw_root = merkle_root(&tx_hashes);

    merkle_root(&[raw_root, byte32_to_arr(witness_root)])
}

pub fn compute_header_hash(header: blockchain::HeaderReader<'_>) -> [u8; 32] {
    blake2b_256(header.as_slice())
}

#[derive(Debug, Clone)]
pub struct VoteResult {
    pub proposal: Proposal,
    pub yes_vote: u64,
    pub no_vote: u64,
    pub passed: bool,
}

fn find_proposal(blocks: BlockVecReader<'_>, proposal_script: &Script) -> Option<Proposal> {
    let first_block = blocks.get(0)?;
    for tx in first_block.transactions().iter() {
        let raw = tx.raw();
        let outputs = raw.outputs();
        let outputs_data = raw.outputs_data();
        for i in 0..outputs.len() {
            let output = outputs.get(i).expect("should exist");
            if let Some(type_script) = output.type_().to_opt() {
                if type_script.as_slice() == proposal_script.as_slice() {
                    let data = outputs_data.get(i)?;
                    return Proposal::from_compatible_slice(data.raw_data()).ok();
                }
            }
        }
    }
    None
}

// main entry to count vote.
pub fn count_vote(blocks: BlockVecReader<'_>, proposal_script: Script) -> VoteResult {
    #[cfg(feature = "profiling")]
    println!("cycle-tracker-report-start: count-vote");

    let proposal = match find_proposal(blocks, &proposal_script) {
        Some(p) => p,
        None => {
            return VoteResult {
                proposal: Proposal::default(),
                yes_vote: 0,
                no_vote: 0,
                passed: false,
            };
        }
    };

    let vote_code_hash = proposal.as_reader().vote_cell_code_hash();
    let vote_hash_type = proposal.as_reader().vote_cell_hash_type();
    let minimal_req = u64::from_le_bytes(
        proposal
            .as_reader()
            .minimal_requirement()
            .as_slice()
            .try_into()
            .expect("Uint64 is 8 bytes"),
    );

    // blake160 = first 20 bytes of blake2b_256; used to verify vote cell args
    let proposal_blake160: [u8; 20] = {
        let hash = blake2b_256(proposal_script.as_slice());
        let mut b = [0u8; 20];
        b.copy_from_slice(&hash[..20]);
        b
    };

    // voter lock hash -> (direction: 0=NO / 1=YES, amount in shannon)
    // see Map in spec
    let mut vote_map: BTreeMap<[u8; 32], (u8, u64)> = BTreeMap::new();
    // DAO deposit outpoint (36 bytes) -> voter lock hash
    // see Map2 in spec
    let mut dao_outpoint_to_voter: BTreeMap<[u8; 36], [u8; 32]> = BTreeMap::new();

    for i in 0..blocks.len() {
        let block = blocks.get(i).expect("should exist");

        for tx in block.transactions().iter() {
            let raw = tx.raw();

            // Invalidate votes whose DAO deposits are spent by this transaction's inputs.
            for input in raw.inputs().iter() {
                let op_bytes: [u8; 36] = input
                    .previous_output()
                    .as_slice()
                    .try_into()
                    .expect("OutPoint is always 36 bytes");
                if let Some(voter_lock_hash) = dao_outpoint_to_voter.remove(&op_bytes) {
                    vote_map.remove(&voter_lock_hash);
                }
            }

            // Record new vote cells found in this transaction's outputs.
            let outputs = raw.outputs();
            let outputs_data = raw.outputs_data();
            let cell_deps = raw.cell_deps();

            for j in 0..outputs.len() {
                let output = outputs.get(j).expect("should exist");
                let type_script = match output.type_().to_opt() {
                    Some(t) => t,
                    None => continue,
                };

                if type_script.code_hash().as_slice() != vote_code_hash.as_slice() {
                    continue;
                }
                if type_script.hash_type().as_slice() != vote_hash_type.as_slice() {
                    continue;
                }
                if type_script.args().raw_data() != &proposal_blake160[..] {
                    continue;
                }

                let data = match outputs_data.get(j) {
                    Some(d) => d,
                    None => continue,
                };

                let vote = match Vote::from_compatible_slice(data.raw_data()) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                let voter_lock_hash = blake2b_256(output.lock().as_slice());
                let direction = vote.as_reader().vote().as_slice()[0];
                let amount = u64::from_le_bytes(
                    vote.as_reader()
                        .amount()
                        .as_slice()
                        .try_into()
                        .expect("Uint64 is 8 bytes"),
                );

                for idx_reader in vote.as_reader().dao_index().iter() {
                    let idx =
                        u16::from_le_bytes(idx_reader.as_slice().try_into().unwrap()) as usize;
                    if let Some(cell_dep) = cell_deps.get(idx) {
                        let op_bytes: [u8; 36] = cell_dep
                            .out_point()
                            .as_slice()
                            .try_into()
                            .expect("OutPoint is always 36 bytes");
                        dao_outpoint_to_voter.insert(op_bytes, voter_lock_hash);
                    }
                }

                vote_map.insert(voter_lock_hash, (direction, amount));
            }
        }
    }

    let mut yes_vote: u64 = 0;
    let mut no_vote: u64 = 0;
    for (_, (direction, amount)) in &vote_map {
        if *direction == 1 {
            yes_vote = yes_vote.saturating_add(*amount);
        } else {
            no_vote = no_vote.saturating_add(*amount);
        }
    }

    let passed = yes_vote > no_vote && yes_vote.saturating_add(no_vote) > minimal_req;

    #[cfg(feature = "profiling")]
    println!("cycle-tracker-report-end: count-vote");

    VoteResult {
        proposal,
        yes_vote,
        no_vote,
        passed,
    }
}

pub fn verify_block_integrity(
    blocks: BlockVecReader<'_>,
    witness_root: blockchain::Byte32VecReader<'_>,
) -> Result<(), Error> {
    if blocks.is_empty() {
        return Err(Error::EmptyBlocks);
    }

    #[cfg(feature = "profiling")]
    println!("cycle-tracker-report-start: block");
    for i in 1..blocks.len() {
        let prev_block = blocks.get(i - 1).expect("should exist");
        let current_block = blocks.get(i).expect("should exist");
        let prev_hash = header_hash(prev_block.header());
        let parent_hash = byte32_to_arr(current_block.header().raw().parent_hash());
        if prev_hash != parent_hash {
            return Err(Error::ParentHashMismatch { block_index: i });
        }
    }
    #[cfg(feature = "profiling")]
    println!("cycle-tracker-report-end: block");

    #[cfg(feature = "profiling")]
    println!("cycle-tracker-report-start: transaction_root");
    for i in 0..blocks.len() {
        let block = blocks.get(i).expect("should exist");
        let expected_root = byte32_to_arr(block.header().raw().transactions_root());
        let wr = witness_root
            .get(i)
            .expect("witness_root index out of bounds");
        let actual_root = calc_transactions_root(block, wr);
        if expected_root != actual_root {
            return Err(Error::TransactionsRootMismatch { block_index: i });
        }
    }
    #[cfg(feature = "profiling")]
    println!("cycle-tracker-report-end: transaction_root");

    Ok(())
}

pub fn prepare_guest_program_arguments(bytes: &[u8]) -> GuestProgramArguments {
    let blocks = BlockVec::from_compatible_slice(bytes).expect("invalid block data");

    let mut all_witness_root: Vec<blockchain::Byte32> = Vec::new();
    for i in 0..blocks.len() {
        let block = blocks.get(i).expect("should exist");
        let hashes: Vec<[u8; 32]> = block
            .as_reader()
            .transactions()
            .iter()
            .map(|tx| witness_hash(&tx))
            .collect();
        all_witness_root.push(merkle_root(&hashes).into());
    }

    let witness_root = all_witness_root
        .into_iter()
        .fold(blockchain::Byte32Vec::new_builder(), |b, h| b.push(h))
        .build();

    let blocks_field = blockchain::Bytes::new_builder()
        .set(
            blocks
                .as_slice()
                .iter()
                .map(|b| blockchain::Byte::new(*b))
                .collect(),
        )
        .build();

    GuestProgramArguments::new_builder()
        .blocks(blocks_field)
        .witness_root(witness_root)
        .build()
}
