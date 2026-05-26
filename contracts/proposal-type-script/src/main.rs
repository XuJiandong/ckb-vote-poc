#![no_std]
#![no_main]

use alloc::vec::Vec;

ckb_std::entry!(program_entry);
ckb_std::default_alloc!(16384, 1258306, 64);

use ckb_hash::new_blake2b;
use ckb_std::{
    ckb_constants::Source,
    high_level::{
        QueryIter, load_cell_data, load_cell_type_hash, load_header, load_script, load_witness_args,
    },
    type_id::check_type_id,
};
use ckb_vote_types::molecules::types::ProposalWitness;
use molecule::prelude::{Entity, Reader};
use sp1_verifier::PlonkVerifier;

#[repr(i8)]
enum Error {
    AppearsOnBothSides = 1,
    TypeIdInvalid,
    ArgsInvalid,
    WitnessInvalid,
    ProofVerifyFailed,
    ProposalMismatch,
    HeaderDepMissing,
    HeaderMismatch,
    ScriptMismatch,
    NotPassed,
}

pub fn program_entry() -> i8 {
    match run() {
        Ok(()) => 0,
        Err(e) => e as i8,
    }
}

fn run() -> Result<(), Error> {
    let script = load_script().map_err(|_| Error::ArgsInvalid)?;
    let args = script.args().raw_data().to_vec();
    // args must be at least 20 (Type ID) + 32 (SP1 VK hash) = 52 bytes
    if args.len() < 52 {
        return Err(Error::ArgsInvalid);
    }

    let script_hash = {
        let mut h = [0u8; 32];
        let mut b = new_blake2b();
        b.update(script.as_slice());
        b.finalize(&mut h);
        h
    };

    // Determine if the script is in inputs, outputs, or both.
    let in_inputs =
        QueryIter::new(load_cell_type_hash, Source::Input).any(|h| h == Some(script_hash));
    let in_outputs =
        QueryIter::new(load_cell_type_hash, Source::Output).any(|h| h == Some(script_hash));

    if in_inputs && in_outputs {
        return Err(Error::AppearsOnBothSides);
    }

    if in_outputs {
        // Creation: verify Type ID (first 20 bytes of args)
        check_type_id(0, 20).map_err(|_| Error::TypeIdInvalid)?;
        return Ok(());
    }

    // Consumption: verify SP1 PLONK proof and public values.
    // Find the input index of this script so we can load the right witness + cell data.
    let input_index = QueryIter::new(load_cell_type_hash, Source::Input)
        .position(|h| h == Some(script_hash))
        .ok_or(Error::ArgsInvalid)?;

    let witness =
        load_witness_args(input_index, Source::Input).map_err(|_| Error::WitnessInvalid)?;

    let raw_witness: Vec<u8> = witness
        .output_type()
        .to_opt()
        .ok_or(Error::WitnessInvalid)?
        .raw_data()
        .to_vec();

    let proposal_witness =
        ProposalWitness::from_slice(&raw_witness).map_err(|_| Error::WitnessInvalid)?;
    let proof_bytes: Vec<u8> = proposal_witness.as_reader().proof().raw_data().to_vec();
    let public_values_bytes: Vec<u8> = proposal_witness
        .as_reader()
        .public_values()
        .as_slice()
        .to_vec();

    // Build the 0x-prefixed hex VK hash string required by PlonkVerifier::verify.
    let vk_hash_raw = &args[20..52];
    let vk_hash_hex = {
        let encoded = hex::encode(vk_hash_raw);
        let mut s = alloc::string::String::with_capacity(2 + encoded.len());
        s.push_str("0x");
        s.push_str(&encoded);
        s
    };

    PlonkVerifier::verify(
        &proof_bytes,
        &public_values_bytes,
        &vk_hash_hex,
        &sp1_verifier::PLONK_VK_BYTES,
    )
    .map_err(|_| Error::ProofVerifyFailed)?;

    let pv = proposal_witness.public_values();

    // proposal field must match cell data.
    let cell_data =
        load_cell_data(input_index, Source::Input).map_err(|_| Error::ProposalMismatch)?;
    if pv.as_reader().proposal().as_slice() != cell_data.as_slice() {
        return Err(Error::ProposalMismatch);
    }

    // header_deps[0] = start block, header_deps[1] = end block.
    let start_header = load_header(0, Source::HeaderDep).map_err(|_| Error::HeaderDepMissing)?;
    let end_header = load_header(1, Source::HeaderDep).map_err(|_| Error::HeaderDepMissing)?;

    let start_hash = {
        let mut h = [0u8; 32];
        let mut b = new_blake2b();
        b.update(start_header.as_slice());
        b.finalize(&mut h);
        h
    };
    let end_hash = {
        let mut h = [0u8; 32];
        let mut b = new_blake2b();
        b.update(end_header.as_slice());
        b.finalize(&mut h);
        h
    };

    if pv.as_reader().start_block_hash().as_slice() != start_hash {
        return Err(Error::HeaderMismatch);
    }
    if pv.as_reader().end_block_hash().as_slice() != end_hash {
        return Err(Error::HeaderMismatch);
    }

    // proposal_script must match the current script.
    if pv.as_reader().proposal_script().as_slice() != script.as_slice() {
        return Err(Error::ScriptMismatch);
    }

    // passed must be 1.
    if pv.as_reader().passed().as_slice()[0] != 1 {
        return Err(Error::NotPassed);
    }

    Ok(())
}
