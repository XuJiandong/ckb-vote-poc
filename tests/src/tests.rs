use ckb_hash::blake2b_256;
use ckb_testtool::{
    builtin::ALWAYS_SUCCESS,
    ckb_types::{
        bytes::Bytes,
        core::{ScriptHashType, TransactionBuilder},
        packed::*,
        prelude::*,
    },
    context::Context,
};
use ckb_vote_types::molecules::{
    blockchain::{Byte as MolByte, Uint64 as MolUint64},
    types::{Uint16, Uint16Vec, Vote},
};

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
