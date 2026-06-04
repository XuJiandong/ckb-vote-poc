use ckb_hash::{blake2b_256, new_blake2b};
use ckb_testtool::{
    builtin::ALWAYS_SUCCESS,
    ckb_types::{
        bytes::Bytes,
        core::{
            EpochNumberWithFraction, HeaderBuilder, HeaderView, ScriptHashType, TransactionBuilder,
        },
        packed::*,
        prelude::*,
    },
    context::Context,
};
use ckb_vote_types::molecules::{
    blockchain::{
        Byte as MolByte, Byte32 as MolByte32, Bytes as MolBytes, Script as MolScript,
        Uint32 as MolUint32, Uint64 as MolUint64,
    },
    types::{Proposal, ProposalWitness, PublicValues, Uint16, Uint16Vec, Vote},
};
use molecule::prelude::{Builder, Entity};

// Nervos DAO genesis type script code hash (RFC 0024).
const DAO_CODE_HASH: [u8; 32] = [
    0x82, 0xd7, 0x6d, 0x1b, 0x75, 0xfe, 0x2f, 0xd9, 0xa2, 0x7d, 0xfb, 0xaa, 0x65, 0xa0, 0x39, 0x22,
    0x1a, 0x38, 0x0d, 0x76, 0xc9, 0x26, 0xf3, 0x78, 0xd3, 0xf8, 0x1c, 0xf3, 0xe7, 0xe1, 0x3f, 0x2e,
];

fn blake160(data: &[u8]) -> [u8; 20] {
    let hash = blake2b_256(data);
    let mut result = [0u8; 20];
    result.copy_from_slice(&hash[..20]);
    result
}

const VK_HASH: [u8; 32] = [0x42u8; 32];

fn calc_type_id(input: &CellInput, output_index: u64) -> [u8; 20] {
    let mut hasher = new_blake2b();
    hasher.update(input.as_slice());
    hasher.update(&output_index.to_le_bytes());
    let mut hash = [0u8; 32];
    hasher.finalize(&mut hash);
    let mut type_id = [0u8; 20];
    type_id.copy_from_slice(&hash[..20]);
    type_id
}

fn deterministic_out_point(tag: u8) -> OutPoint {
    OutPoint::new_builder()
        .tx_hash(Byte32::from([tag; 32]))
        .index(tag as u32)
        .build()
}

fn proposal_type_script_args(type_id: [u8; 20]) -> Bytes {
    let mut args = Vec::with_capacity(52);
    args.extend_from_slice(&type_id);
    args.extend_from_slice(&VK_HASH);
    Bytes::from(args)
}

fn sample_proposal() -> Proposal {
    Proposal::new_builder()
        .duration(MolUint32::from(100u32.to_le_bytes()))
        .vote_cell_code_hash(MolByte32::from([0x11u8; 32]))
        .vote_cell_hash_type(MolByte::new(1))
        .description(MolBytes::new_builder().build())
        .receiver(MolScript::new_builder().build())
        .amount(MolUint64::from(1000u64.to_le_bytes()))
        .minimal_requirement(MolUint64::from(500u64.to_le_bytes()))
        .build()
}

fn header_hash(header: &HeaderView) -> [u8; 32] {
    blake2b_256(header.data().as_slice())
}

fn mol_script(script: &Script) -> MolScript {
    MolScript::from_slice(script.as_slice()).expect("valid script molecule")
}

fn to_mol_bytes(data: &[u8]) -> MolBytes {
    MolBytes::new_builder()
        .set(data.iter().map(|b| MolByte::new(*b)).collect())
        .build()
}

/// Test creating a vote cell (casting a YES vote).
///
/// Transaction layout:
///   cell_deps[0]: proposal cell (mock — always-success type script)
///   cell_deps[1]: DAO deposit cell (voter_lock, Nervos DAO type, capacity = 500)
///   cell_deps[2+]: script code cells injected by context.complete_tx
///
///   inputs[0]:  funding cell (voter_lock, capacity = 1000)
///   outputs[0]: vote cell (voter_lock, vote type script, Vote data)
///   outputs[1]: change cell (voter_lock)
#[test]
fn test_vote_type_script() {
    let mut context = Context::default();

    // Deploy vote-type-script binary (must be built first via `make build`).
    let vote_out_point = context.deploy_cell_by_name("vote-type-script");

    // Deploy always-success script used as voter lock and as mock proposal type.
    let always_success_op = context.deploy_cell(ALWAYS_SUCCESS.clone());

    // Voter lock: always-success with args [0x01] to distinguish from other cells.
    let voter_lock = context
        .build_script(&always_success_op, Bytes::from(vec![0x01u8]))
        .expect("voter lock");

    // Mock proposal type script: always-success with args [0x02].
    // We use its full molecule bytes to compute the blake160 that goes into vote args.
    let proposal_type_script = context
        .build_script(&always_success_op, Bytes::from(vec![0x02u8]))
        .expect("proposal type script");
    let proposal_blake160 = blake160(proposal_type_script.as_slice());

    // Vote type script: args = blake160(proposal_type_script.as_slice()).
    let vote_type_script = context
        .build_script(&vote_out_point, Bytes::from(proposal_blake160.to_vec()))
        .expect("vote type script");

    // ─── cell_dep[0]: proposal cell ───────────────────────────────────────────
    let proposal_lock = context
        .build_script(&always_success_op, Bytes::new())
        .expect("proposal lock");
    let proposal_cell_op = context.create_cell(
        CellOutput::new_builder()
            .capacity(1000u64)
            .lock(proposal_lock)
            .type_(Some(proposal_type_script).pack())
            .build(),
        Bytes::new(),
    );
    let proposal_dep = CellDep::new_builder().out_point(proposal_cell_op).build();

    // ─── cell_dep[1]: DAO deposit cell ────────────────────────────────────────
    // The vote script only reads the cell's metadata (lock, type, capacity); the
    // DAO script binary is never executed, so we only need the correct code_hash.
    let dao_type_script = Script::new_builder()
        .code_hash(Byte32::from_slice(&DAO_CODE_HASH).unwrap())
        .hash_type(ScriptHashType::Type)
        .args(Bytes::new().pack())
        .build();
    let dao_deposit_op = context.create_cell(
        CellOutput::new_builder()
            .capacity(500u64)
            .lock(voter_lock.clone())
            .type_(Some(dao_type_script).pack())
            .build(),
        Bytes::new(),
    );
    let dao_dep = CellDep::new_builder().out_point(dao_deposit_op).build();

    // ─── input[0]: funding cell ───────────────────────────────────────────────
    let input_op = context.create_cell(
        CellOutput::new_builder()
            .capacity(1000u64)
            .lock(voter_lock.clone())
            .build(),
        Bytes::new(),
    );
    let input = CellInput::new_builder().previous_output(input_op).build();

    // ─── Vote cell data ───────────────────────────────────────────────────────
    // YES vote, amount = 500 (shannons), dao_index = [1] → cell_dep[1]
    let vote = Vote::new_builder()
        .vote(MolByte::new(1))
        .amount(MolUint64::from(500u64.to_le_bytes()))
        .dao_index(
            Uint16Vec::new_builder()
                .push(Uint16::from(1u16.to_le_bytes()))
                .build(),
        )
        .build();

    // ─── outputs ──────────────────────────────────────────────────────────────
    let vote_output = CellOutput::new_builder()
        .capacity(500u64)
        .lock(voter_lock.clone())
        .type_(Some(vote_type_script).pack())
        .build();
    let change_output = CellOutput::new_builder()
        .capacity(499u64)
        .lock(voter_lock)
        .build();

    // Build transaction. cell_dep indices 0 and 1 are proposal and DAO;
    // context.complete_tx appends the script code deps after them.
    let tx = TransactionBuilder::default()
        .cell_dep(proposal_dep) // index 0
        .cell_dep(dao_dep) // index 1
        .input(input)
        .output(vote_output)
        .output(change_output)
        .output_data(Bytes::from(vote.as_slice().to_vec()).pack())
        .output_data(Bytes::new().pack())
        .build();
    let tx = context.complete_tx(tx);

    let cycles = context
        .verify_tx(&tx, 10_000_000)
        .expect("pass verification");
    println!("consume cycles: {}", cycles);
}

#[test]
fn test_vote_type_script_reject_empty_dao_index() {
    let mut context = Context::default();

    let vote_out_point = context.deploy_cell_by_name("vote-type-script");
    let always_success_op = context.deploy_cell(ALWAYS_SUCCESS.clone());

    let voter_lock = context
        .build_script(&always_success_op, Bytes::from(vec![0x01u8]))
        .expect("voter lock");

    let proposal_type_script = context
        .build_script(&always_success_op, Bytes::from(vec![0x02u8]))
        .expect("proposal type script");
    let proposal_blake160 = blake160(proposal_type_script.as_slice());
    let vote_type_script = context
        .build_script(&vote_out_point, Bytes::from(proposal_blake160.to_vec()))
        .expect("vote type script");

    let proposal_lock = context
        .build_script(&always_success_op, Bytes::new())
        .expect("proposal lock");
    let proposal_cell_op = context.create_cell(
        CellOutput::new_builder()
            .capacity(1000u64)
            .lock(proposal_lock)
            .type_(Some(proposal_type_script).pack())
            .build(),
        Bytes::new(),
    );
    let proposal_dep = CellDep::new_builder().out_point(proposal_cell_op).build();

    let input_op = context.create_cell(
        CellOutput::new_builder()
            .capacity(1000u64)
            .lock(voter_lock.clone())
            .build(),
        Bytes::new(),
    );
    let input = CellInput::new_builder().previous_output(input_op).build();

    let vote = Vote::new_builder()
        .vote(MolByte::new(1))
        .amount(MolUint64::from(0u64.to_le_bytes()))
        .dao_index(Uint16Vec::new_builder().build())
        .build();

    let vote_output = CellOutput::new_builder()
        .capacity(500u64)
        .lock(voter_lock.clone())
        .type_(Some(vote_type_script).pack())
        .build();
    let change_output = CellOutput::new_builder()
        .capacity(499u64)
        .lock(voter_lock)
        .build();

    let tx = TransactionBuilder::default()
        .cell_dep(proposal_dep)
        .input(input)
        .output(vote_output)
        .output(change_output)
        .output_data(Bytes::from(vote.as_slice().to_vec()).pack())
        .output_data(Bytes::new().pack())
        .build();
    let tx = context.complete_tx(tx);

    let result = context.verify_tx(&tx, 10_000_000);
    let err = result.expect_err("empty dao_index should fail");
    let err_msg = format!("{err:?}");
    assert!(
        err_msg.contains("error code 5"),
        "expected DaoDepInvalid (exit code 5), got: {err_msg}"
    );
}

/// Test consuming a vote cell (recycling CKB).
/// The vote type script should allow this unconditionally.
#[test]
fn test_vote_type_script_consume() {
    let mut context = Context::default();

    let vote_out_point = context.deploy_cell_by_name("vote-type-script");
    let always_success_op = context.deploy_cell(ALWAYS_SUCCESS.clone());

    let voter_lock = context
        .build_script(&always_success_op, Bytes::from(vec![0x01u8]))
        .expect("voter lock");

    // Give the vote type script some valid-looking args (doesn't matter for consume path).
    let dummy_args = Bytes::from(vec![0u8; 20]);
    let vote_type_script = context
        .build_script(&vote_out_point, dummy_args)
        .expect("vote type script");

    // Create a pre-existing vote cell.
    let vote = Vote::new_builder()
        .vote(MolByte::new(1))
        .amount(MolUint64::from(0u64.to_le_bytes()))
        .build();
    let vote_cell_op = context.create_cell(
        CellOutput::new_builder()
            .capacity(500u64)
            .lock(voter_lock.clone())
            .type_(Some(vote_type_script).pack())
            .build(),
        Bytes::from(vote.as_slice().to_vec()),
    );

    let input = CellInput::new_builder()
        .previous_output(vote_cell_op)
        .build();
    let output = CellOutput::new_builder()
        .capacity(499u64)
        .lock(voter_lock)
        .build();

    let tx = TransactionBuilder::default()
        .input(input)
        .output(output)
        .output_data(Bytes::new().pack())
        .build();
    let tx = context.complete_tx(tx);

    let cycles = context
        .verify_tx(&tx, 10_000_000)
        .expect("consume vote cell passes");
    println!("consume cycles: {}", cycles);
}

/// Test creating a proposal cell.
///
/// Transaction layout:
///   inputs[0]:  funding cell (always-success lock)
///   outputs[0]: proposal cell (always-success lock, proposal type script, Proposal data)
///   outputs[1]: change cell (always-success lock)
#[test]
fn test_proposal_type_script_create() {
    let mut context = Context::new_with_deterministic_rng();

    let proposal_out_point = context.deploy_cell_by_name("proposal-type-script");
    let always_success_op = context.deploy_cell(ALWAYS_SUCCESS.clone());

    let proposer_lock = context
        .build_script(&always_success_op, Bytes::from(vec![0x01u8]))
        .expect("proposer lock");

    let funding_op = deterministic_out_point(0x01);
    context.create_cell_with_out_point(
        funding_op.clone(),
        CellOutput::new_builder()
            .capacity(2000u64)
            .lock(proposer_lock.clone())
            .build(),
        Bytes::new(),
    );
    let input = CellInput::new_builder().previous_output(funding_op).build();

    let type_id = calc_type_id(&input, 0);
    let proposal_type_script = context
        .build_script(&proposal_out_point, proposal_type_script_args(type_id))
        .expect("proposal type script");

    let proposal = sample_proposal();
    let proposal_output = CellOutput::new_builder()
        .capacity(1000u64)
        .lock(proposer_lock.clone())
        .type_(Some(proposal_type_script).pack())
        .build();
    let change_output = CellOutput::new_builder()
        .capacity(999u64)
        .lock(proposer_lock)
        .build();

    let tx = TransactionBuilder::default()
        .input(input)
        .output(proposal_output)
        .output(change_output)
        .output_data(Bytes::from(proposal.as_slice().to_vec()).pack())
        .output_data(Bytes::new().pack())
        .build();
    let tx = context.complete_tx(tx);

    let cycles = context
        .verify_tx(&tx, 10_000_000)
        .expect("create proposal cell passes");
    println!("create proposal cycles: {}", cycles);
}

/// Test consuming a proposal cell with a dummy SP1 proof.
///
/// All pre-proof checks should pass; verification fails at `PlonkVerifier::verify`
/// because no valid proof is available yet.
#[test]
fn test_proposal_type_script_consume() {
    let mut context = Context::new_with_deterministic_rng();

    let proposal_out_point = context.deploy_cell_by_name("proposal-type-script");
    let always_success_op = context.deploy_cell(ALWAYS_SUCCESS.clone());

    let always_success_lock = context
        .build_script(&always_success_op, Bytes::from(vec![0x01u8]))
        .expect("proposer lock");

    let type_id = [0x33u8; 20];
    let proposal_type_script = context
        .build_script(&proposal_out_point, proposal_type_script_args(type_id))
        .expect("proposal type script");

    let proposal = sample_proposal();
    let proposal_cell_op = deterministic_out_point(0x02);
    context.create_cell_with_out_point(
        proposal_cell_op.clone(),
        CellOutput::new_builder()
            .capacity(1000u64)
            .lock(always_success_lock.clone())
            .type_(Some(proposal_type_script.clone()).pack())
            .build(),
        Bytes::from(proposal.as_slice().to_vec()),
    );

    let treasury_op = deterministic_out_point(0x03);
    context.create_cell_with_out_point(
        treasury_op.clone(),
        CellOutput::new_builder()
            .capacity(2000u64)
            .lock(always_success_lock.clone())
            .build(),
        Bytes::new(),
    );

    let start_header = HeaderBuilder::default()
        .number(1000)
        .epoch(EpochNumberWithFraction::new(0, 1, 1000))
        .build();
    let end_header = HeaderBuilder::default()
        .number(1100)
        .epoch(EpochNumberWithFraction::new(0, 101, 1000))
        .build();
    context.insert_header(start_header.clone());
    context.insert_header(end_header.clone());

    let public_values = PublicValues::new_builder()
        .proposal(proposal.clone())
        .start_block_hash(MolByte32::from(header_hash(&start_header)))
        .end_block_hash(MolByte32::from(header_hash(&end_header)))
        .proposal_script(mol_script(&proposal_type_script))
        .passed(MolByte::new(1))
        .yes_vote(MolUint64::from(600u64.to_le_bytes()))
        .no_vote(MolUint64::from(100u64.to_le_bytes()))
        .build();

    let proposal_witness = ProposalWitness::new_builder()
        .proof(to_mol_bytes(&[0xde, 0xad, 0xd0]))
        .public_values(public_values)
        .build();

    let witness = WitnessArgs::new_builder()
        .input_type(Some(Bytes::from(proposal_witness.as_slice().to_vec())).pack())
        .build();

    let proposal_input = CellInput::new_builder()
        .previous_output(proposal_cell_op)
        .build();
    let treasury_input = CellInput::new_builder()
        .previous_output(treasury_op)
        .build();

    let receiver_output = CellOutput::new_builder()
        .capacity(1000u64)
        .lock(always_success_lock.clone())
        .build();
    let change_output = CellOutput::new_builder()
        .capacity(1999u64)
        .lock(always_success_lock)
        .build();

    let tx = TransactionBuilder::default()
        .input(proposal_input)
        .input(treasury_input)
        .output(receiver_output)
        .output(change_output)
        .output_data(Bytes::new().pack())
        .output_data(Bytes::new().pack())
        .header_dep(start_header.hash())
        .header_dep(end_header.hash())
        .witness(witness.as_bytes().pack())
        .witness(WitnessArgs::default().as_bytes().pack())
        .build();
    let tx = context.complete_tx(tx);

    let result = context.verify_tx(&tx, 100_000_000);
    let err = result.expect_err("dummy proof should fail at PlonkVerifier::verify");
    let err_msg = format!("{err:?}");
    assert!(
        err_msg.contains("error code 5"),
        "expected ProofVerifyFailed (exit code 5), got: {err_msg}"
    );
}
