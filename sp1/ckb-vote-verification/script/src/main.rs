use ckb_vote_types::molecules::{
    blockchain,
    types::{Proposal, PublicValues},
};
use clap::Parser;
use molecule::prelude::{Builder, Entity};
use sp1_sdk::{
    HashableKey, ProveRequest, Prover, ProverClient, ProvingKey, SP1Stdin, include_elf,
    network::NetworkMode, utils,
};

const ELF: sp1_sdk::Elf = include_elf!("ckb-vote-verification-program");
const VERIFYING_KEY_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../verifying-key.txt");
const PROOF_OUTPUT: &str = "proof-plonk.bin";
const PUBLIC_VALUES_OUTPUT: &str = "public-values.bin";

const BLOCK_DATA: &[u8] = include_bytes!("../../../../crates/verification/tests/blocks.bin");

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    execute: bool,

    #[arg(long)]
    prove: bool,

    #[arg(long)]
    prove_via_network: bool,
}

fn sample_proposal() -> Proposal {
    Proposal::new_builder()
        .vote_cell_code_hash(blockchain::Byte32::from([1u8; 32]))
        .vote_cell_hash_type(blockchain::Byte::new(0))
        .minimal_requirement(blockchain::Uint64::from(100u64.to_le_bytes()))
        .build()
}

fn prepare_stdin() -> SP1Stdin {
    let guest_args = ckb_vote_testtool::generate_from_templates(sample_proposal(), BLOCK_DATA)
        .expect("generate_from_templates");
    let mut stdin = SP1Stdin::new();
    stdin.write_vec(guest_args.as_slice().to_vec());
    stdin
}

fn save_verifying_key(pk: &impl ProvingKey) {
    std::fs::write(VERIFYING_KEY_PATH, pk.verifying_key().bytes32())
        .expect("failed to write verifying key");
}

fn print_public_values(pv_bytes: &[u8]) {
    let pv = PublicValues::from_slice(pv_bytes).expect("invalid PublicValues");
    let pv = pv.as_reader();

    println!(
        "start block hash: 0x{}",
        hex::encode(pv.start_block_hash().raw_data())
    );
    println!(
        "end block hash:   0x{}",
        hex::encode(pv.end_block_hash().raw_data())
    );
    println!(
        "yes_vote:          {}",
        u64::from_le_bytes(pv.yes_vote().raw_data().try_into().unwrap())
    );
    println!(
        "no_vote:           {}",
        u64::from_le_bytes(pv.no_vote().raw_data().try_into().unwrap())
    );
    println!("passed:            {}", u8::from(pv.passed()) != 0);
}

fn assert_passed(pv_bytes: &[u8]) {
    let pv = PublicValues::from_slice(pv_bytes).expect("invalid PublicValues");
    assert_eq!(u8::from(pv.as_reader().passed()), 1);
}

#[tokio::main]
async fn main() {
    utils::setup_logger();

    let cli = Args::parse();

    let mode_count = [cli.execute, cli.prove, cli.prove_via_network]
        .into_iter()
        .filter(|&m| m)
        .count();
    if mode_count != 1 {
        eprintln!("Error: specify exactly one of --execute, --prove, or --prove-via-network");
        std::process::exit(1);
    }

    let stdin = prepare_stdin();

    if cli.execute {
        run_execute(stdin).await;
    } else if cli.prove {
        run_prove(stdin).await;
    } else {
        run_prove_via_network(stdin).await;
    }
}

async fn run_execute(stdin: SP1Stdin) {
    let client = ProverClient::builder().cpu().build().await;
    let pk = client.setup(ELF).await.unwrap();
    save_verifying_key(&pk);

    let (public_values, report) = client.execute(ELF, stdin).await.unwrap();

    let pv_bytes = public_values.as_slice().to_vec();
    print_public_values(&pv_bytes);

    #[cfg(feature = "profiling")]
    {
        let blake2b_cycles = report.cycle_tracker.get("blake2b").unwrap();
        println!(
            "blake2b with {:.0} M instructions ",
            *blake2b_cycles as f64 / 1000.0 / 1000.0
        );
        let block_cycles = report.cycle_tracker.get("block").unwrap();
        println!(
            "block with {:.0} M instructions ",
            *block_cycles as f64 / 1000.0 / 1000.0
        );

        let transaction_root_cycles = report.cycle_tracker.get("transaction_root").unwrap();
        println!(
            "transaction_root with {:.0} M instructions ",
            *transaction_root_cycles as f64 / 1000.0 / 1000.0
        );
        let block_stats_cycles = report.cycle_tracker.get("block-stats").unwrap();
        println!(
            "block-stats with {:.0} M instructions ",
            *block_stats_cycles as f64 / 1000.0 / 1000.0
        );
    }
    println!(
        "executed program with {:.0} M instructions",
        report.total_instruction_count() as f64 / 1000.0 / 1000.0
    );
    assert_passed(&pv_bytes);
    #[cfg(not(feature = "profiling"))]
    let _ = (public_values, report);
}

async fn run_prove(stdin: SP1Stdin) {
    let client = ProverClient::builder().cpu().build().await;
    let pk = client.setup(ELF).await.unwrap();
    save_verifying_key(&pk);

    let proof = client.prove(&pk, stdin).core().await.unwrap();

    let pv_bytes = proof.public_values.as_slice().to_vec();
    print_public_values(&pv_bytes);

    client
        .verify(&proof, pk.verifying_key(), None)
        .expect("verification failed");
    assert_passed(&pv_bytes);
    println!("successfully generated and verified proof!");
}

async fn run_prove_via_network(stdin: SP1Stdin) {
    // NETWORK_PRIVATE_KEY env var must be set to your requester account's private key.
    let client = ProverClient::builder()
        .network_for(NetworkMode::Mainnet)
        .build()
        .await;

    let pk = client.setup(ELF).await.unwrap();
    save_verifying_key(&pk);

    let proof = client.prove(&pk, stdin).plonk().await.unwrap();

    let pv_bytes = proof.public_values.as_slice().to_vec();
    print_public_values(&pv_bytes);

    client
        .verify(&proof, pk.verifying_key(), None)
        .expect("verification failed");
    assert_passed(&pv_bytes);

    std::fs::write(PROOF_OUTPUT, proof.bytes()).expect("failed to save proof");
    std::fs::write(PUBLIC_VALUES_OUTPUT, &pv_bytes).expect("failed to save public values");
    println!("proof saved to {PROOF_OUTPUT}");
    println!("public values saved to {PUBLIC_VALUES_OUTPUT}");
}
