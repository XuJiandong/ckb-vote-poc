#![allow(dead_code)]
#![allow(unused_imports)]

// Import from `core` instead of from `std` since we are in no-std mode
use core::result::Result;

// Import heap related library from `alloc`
// https://doc.rust-lang.org/alloc/index.html
use alloc::format;

use ckb_std::syscalls::debug;
// Import CKB syscalls and structures
// https://docs.rs/ckb-std/
use crate::error::Error;
use ckb_std::syscalls::current_cycles;
use sp1_verifier::PlonkVerifier;

const PROOF: &[u8] = include_bytes!("../proof-plonk.bin");
const PUBLIC_VALUES: &[u8] = include_bytes!("../public-values.bin");
const VK_HASH: &str = include_str!("../verifying-key.txt");

pub fn main() -> Result<(), Error> {
    let vk_hash = VK_HASH.trim();
    // use this to trigger PlonkError::PairingCheckFailed
    // let mut proof = PROOF.to_vec();
    // proof[484..516].fill(0);

    let last = current_cycles();
    PlonkVerifier::verify(PROOF, PUBLIC_VALUES, vk_hash, sp1_verifier::PLONK_VK_BYTES)
        .expect("plonk verify failed");

    let cycles = current_cycles() - last;

    debug(format!(
        "cost of sp1(zkVM) verifying cycles: {:.1} M",
        cycles as f64 / (1000.0 * 1000.0)
    ));

    Ok(())
}
