use ckb_vote_types::molecules::{
    blockchain::{
        self, Block, Byte32, Byte32Vec, CellOutput, CellOutputVec, Header, RawHeader,
        RawTransaction, Script, ScriptOpt, Transaction, TransactionVec, Uint32,
    },
    types::{BlockVec, GuestProgramArguments, Proposal, Vote},
};
use ckb_vote_verification::{blake2b_256, merkle_root, tx_hash, witness_hash};
use molecule::prelude::{Builder, Entity, Reader};

pub type Result<T, E = Error> = core::result::Result<T, E>;

#[derive(Debug)]
pub enum Error {
    Parse(molecule::error::VerificationError),
}

impl From<molecule::error::VerificationError> for Error {
    fn from(e: molecule::error::VerificationError) -> Self {
        Error::Parse(e)
    }
}

fn blake160(data: &[u8]) -> [u8; 20] {
    let hash = blake2b_256(data);
    let mut result = [0u8; 20];
    result.copy_from_slice(&hash[..20]);
    result
}

fn calc_transactions_root(txs: &[Transaction]) -> [u8; 32] {
    let tx_hashes: Vec<[u8; 32]> = txs.iter().map(|tx| tx_hash(&tx.as_reader())).collect();
    let raw_root = merkle_root(&tx_hashes);
    let witness_hashes: Vec<[u8; 32]> =
        txs.iter().map(|tx| witness_hash(&tx.as_reader())).collect();
    let witness_root = merkle_root(&witness_hashes);
    merkle_root(&[raw_root, witness_root])
}

fn to_blockchain_bytes(data: &[u8]) -> blockchain::Bytes {
    blockchain::Bytes::new_builder()
        .set(data.iter().map(|b| blockchain::Byte::new(*b)).collect())
        .build()
}

fn build_proposal_tx(proposal: &Proposal, proposal_script: &Script) -> Transaction {
    let lock = Script::new_builder().build();
    let output = CellOutput::new_builder()
        .lock(lock)
        .type_(
            ScriptOpt::new_builder()
                .set(Some(proposal_script.clone()))
                .build(),
        )
        .build();
    let raw_tx = RawTransaction::new_builder()
        .outputs(CellOutputVec::new_builder().push(output).build())
        .outputs_data(
            blockchain::BytesVec::new_builder()
                .push(to_blockchain_bytes(proposal.as_slice()))
                .build(),
        )
        .build();
    Transaction::new_builder().raw(raw_tx).build()
}

fn build_vote_tx(vote_type_script: &Script, voter_index: usize) -> Transaction {
    // Use a unique args per voter so each vote has a distinct voter_lock_hash in vote_map.
    let mut voter_args = [0u8; 8];
    voter_args.copy_from_slice(&voter_index.to_le_bytes());
    let lock = Script::new_builder()
        .args(to_blockchain_bytes(&voter_args))
        .build();
    let output = CellOutput::new_builder()
        .lock(lock)
        .type_(
            ScriptOpt::new_builder()
                .set(Some(vote_type_script.clone()))
                .build(),
        )
        .build();
    let vote = Vote::new_builder()
        .vote(blockchain::Byte::new(1)) // YES
        .amount(blockchain::Uint64::from(100u64.to_le_bytes()))
        .build();
    let raw_tx = RawTransaction::new_builder()
        .outputs(CellOutputVec::new_builder().push(output).build())
        .outputs_data(
            blockchain::BytesVec::new_builder()
                .push(to_blockchain_bytes(vote.as_slice()))
                .build(),
        )
        .build();
    Transaction::new_builder().raw(raw_tx).build()
}

fn rebuild_block_with_extra_tx(template: Block, extra_tx: Transaction) -> Block {
    let old_txs = template.transactions();
    let mut txs: Vec<Transaction> = (0..old_txs.len())
        .map(|i| old_txs.get(i).unwrap())
        .collect();
    txs.push(extra_tx);

    let new_transactions_root = calc_transactions_root(&txs);
    let tx_vec = txs
        .into_iter()
        .fold(TransactionVec::new_builder(), |b, tx| b.push(tx))
        .build();

    let old_raw = template.header().raw();
    let new_raw = RawHeader::new_builder()
        .version(old_raw.version())
        .compact_target(old_raw.compact_target())
        .timestamp(old_raw.timestamp())
        .number(old_raw.number())
        .epoch(old_raw.epoch())
        .parent_hash(old_raw.parent_hash())
        .transactions_root(Byte32::from(new_transactions_root))
        .proposals_hash(old_raw.proposals_hash())
        .extra_hash(old_raw.extra_hash())
        .dao(old_raw.dao())
        .build();
    let new_header = Header::new_builder()
        .raw(new_raw)
        .nonce(template.header().nonce())
        .build();

    Block::new_builder()
        .header(new_header)
        .uncles(template.uncles())
        .transactions(tx_vec)
        .proposals(template.proposals())
        .build()
}

/// Converts `template_blocks` into the molecule type `BlockVec`.
/// All available template blocks are used (up to `block_vec.len()`). The proposal's `duration`
/// field is set to `num_blocks - 1` so the embedded proposal is always consistent with the block
/// window, matching the exact invariant required by `count_vote` (`blocks.len() == duration + 1`).
/// For each block, one transaction is appended: the first block gets a transaction containing a
/// proposal cell (holding the updated `proposal`), and each subsequent block gets a transaction
/// containing a single vote cell.
/// After construction, `parent_hash` fields are recalculated and updated throughout the chain.
/// The result is a valid input to `count_vote`.
pub fn generate_from_templates(
    proposal: Proposal,
    template_blocks: &[u8],
) -> Result<GuestProgramArguments> {
    let block_vec = BlockVec::from_compatible_slice(template_blocks)?;

    let num_blocks = block_vec.len();

    // Stamp duration = num_blocks - 1 so the embedded proposal is consistent with the block window.
    let proposal = proposal
        .as_builder()
        .duration(Uint32::from(((num_blocks - 1) as u32).to_le_bytes()))
        .build();

    // Build a deterministic proposal_script whose code_hash is derived from the proposal content.
    let proposal_script = Script::new_builder()
        .code_hash(Byte32::from(blake2b_256(proposal.as_slice())))
        .build();

    let proposal_blake160 = blake160(proposal_script.as_slice());
    let vote_type_script = Script::new_builder()
        .code_hash(proposal.as_reader().vote_cell_code_hash().to_entity())
        .hash_type(proposal.as_reader().vote_cell_hash_type().to_entity())
        .args(to_blockchain_bytes(&proposal_blake160))
        .build();

    // Append one transaction per block.
    let mut blocks: Vec<Block> = Vec::with_capacity(num_blocks);
    for i in 0..num_blocks {
        let template = block_vec.get(i).unwrap();
        let extra_tx = if i == 0 {
            build_proposal_tx(&proposal, &proposal_script)
        } else {
            build_vote_tx(&vote_type_script, i)
        };
        blocks.push(rebuild_block_with_extra_tx(template, extra_tx));
    }

    // Recompute parent_hash fields throughout the chain.
    for i in 1..blocks.len() {
        let parent_hash = blake2b_256(blocks[i - 1].header().as_slice());
        let old = blocks[i].clone();
        let old_raw = old.header().raw();
        let new_raw = RawHeader::new_builder()
            .version(old_raw.version())
            .compact_target(old_raw.compact_target())
            .timestamp(old_raw.timestamp())
            .number(old_raw.number())
            .epoch(old_raw.epoch())
            .parent_hash(Byte32::from(parent_hash))
            .transactions_root(old_raw.transactions_root())
            .proposals_hash(old_raw.proposals_hash())
            .extra_hash(old_raw.extra_hash())
            .dao(old_raw.dao())
            .build();
        let new_header = Header::new_builder()
            .raw(new_raw)
            .nonce(old.header().nonce())
            .build();
        blocks[i] = Block::new_builder()
            .header(new_header)
            .uncles(old.uncles())
            .transactions(old.transactions())
            .proposals(old.proposals())
            .build();
    }

    // Compute per-block witness roots for GuestProgramArguments.
    let witness_roots: Vec<Byte32> = blocks
        .iter()
        .map(|block| {
            let hashes: Vec<[u8; 32]> = (0..block.transactions().len())
                .map(|i| witness_hash(&block.transactions().get(i).unwrap().as_reader()))
                .collect();
            Byte32::from(merkle_root(&hashes))
        })
        .collect();
    let witness_root_vec = witness_roots
        .into_iter()
        .fold(Byte32Vec::new_builder(), |b, h| b.push(h))
        .build();

    let new_block_vec = blocks
        .into_iter()
        .fold(BlockVec::new_builder(), |b, block| b.push(block))
        .build();

    Ok(GuestProgramArguments::new_builder()
        .blocks(to_blockchain_bytes(new_block_vec.as_slice()))
        .witness_root(witness_root_vec)
        .proposal_script(proposal_script)
        .build())
}
