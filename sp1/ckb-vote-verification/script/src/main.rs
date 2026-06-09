use ckb_vote_types::molecules::{
    blockchain,
    types::{BlockVec, Proposal, PublicValues},
};
use clap::Parser;
use molecule::prelude::{Builder, Entity};
use sp1_sdk::{
    HashableKey, ProveRequest, Prover, ProverClient, ProvingKey, SP1Stdin, include_elf,
    network::NetworkMode, utils,
};
use std::path::PathBuf;

const ELF: sp1_sdk::Elf = include_elf!("ckb-vote-verification-program");
const VERIFYING_KEY_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../verifying-key.txt");
const PROOF_OUTPUT: &str = "proof-plonk.bin";
const PUBLIC_VALUES_OUTPUT: &str = "public-values.bin";
const DEFAULT_OUTPUT_DIR: &str = ".";

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    execute: bool,

    #[arg(long)]
    prove: bool,

    #[arg(long)]
    prove_via_network: bool,

    /// Use a hardcoded sample proposal instead of reading from block data
    #[arg(long)]
    mock: bool,

    /// Path to a file containing block data
    #[arg(long)]
    input: PathBuf,

    /// Output folder for proof-plonk.bin and public-values.bin (only used with --prove-via-network)
    #[arg(long, default_value = DEFAULT_OUTPUT_DIR)]
    output: PathBuf,

    /// Transaction hash of the proposal cell in the first block (required without --mock)
    #[arg(long)]
    proposal_tx_hash: Option<String>,

    /// Output index of the proposal cell (required without --mock)
    #[arg(long)]
    proposal_index: Option<u32>,
}

fn sample_proposal() -> Proposal {
    Proposal::new_builder()
        .vote_cell_code_hash(blockchain::Byte32::from([1u8; 32]))
        .vote_cell_hash_type(blockchain::Byte::new(0))
        .minimal_requirement(blockchain::Uint64::from(0u64.to_le_bytes()))
        .build()
}

fn find_proposal_in_first_block(
    block_data: &[u8],
    tx_hash_hex: &str,
    output_index: u32,
) -> Proposal {
    let tx_hash_bytes: [u8; 32] = hex::decode(tx_hash_hex.trim_start_matches("0x"))
        .unwrap_or_else(|e| {
            eprintln!("Error: invalid --proposal-tx-hash: {e}");
            std::process::exit(1);
        })
        .try_into()
        .unwrap_or_else(|_| {
            eprintln!("Error: --proposal-tx-hash must be 32 bytes (64 hex chars)");
            std::process::exit(1);
        });

    let block_vec = BlockVec::from_compatible_slice(block_data).unwrap_or_else(|e| {
        eprintln!("Error: invalid block data: {e}");
        std::process::exit(1);
    });
    let first_block = block_vec.get(0).unwrap_or_else(|| {
        eprintln!("Error: block data contains no blocks");
        std::process::exit(1);
    });

    let txs = first_block.transactions();
    for i in 0..txs.len() {
        let tx = txs.get(i).unwrap();
        let hash = ckb_vote_verification::tx_hash(&tx.as_reader());
        if hash == tx_hash_bytes {
            let data = tx
                .raw()
                .outputs_data()
                .get(output_index as usize)
                .unwrap_or_else(|| {
                    eprintln!("Error: --proposal-index {output_index} out of range");
                    std::process::exit(1);
                });
            return Proposal::from_slice(&data.raw_data()).unwrap_or_else(|e| {
                eprintln!("Error: failed to parse Proposal from cell data: {e}");
                std::process::exit(1);
            });
        }
    }

    eprintln!("Error: transaction not found in first block: {tx_hash_hex}");
    std::process::exit(1);
}

fn prepare_stdin(block_data: &[u8], proposal: Proposal) -> SP1Stdin {
    let guest_args = ckb_vote_testtool::generate_from_templates(proposal, block_data)
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

    let block_data = std::fs::read(&cli.input).unwrap_or_else(|e| {
        eprintln!(
            "Error: failed to read input file {}: {e}",
            cli.input.display()
        );
        std::process::exit(1);
    });

    let proposal = if cli.mock {
        sample_proposal()
    } else {
        let tx_hash = cli.proposal_tx_hash.as_deref().unwrap_or_else(|| {
            eprintln!("Error: --proposal-tx-hash is required when --mock is not set");
            std::process::exit(1);
        });
        let index = cli.proposal_index.unwrap_or_else(|| {
            eprintln!("Error: --proposal-index is required when --mock is not set");
            std::process::exit(1);
        });
        find_proposal_in_first_block(&block_data, tx_hash, index)
    };

    let stdin = prepare_stdin(&block_data, proposal);

    if cli.execute {
        run_execute(stdin).await;
    } else if cli.prove {
        run_prove(stdin).await;
    } else {
        run_prove_via_network(stdin, &cli.output).await;
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

async fn run_prove_via_network(stdin: SP1Stdin, output_dir: &PathBuf) {
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

    let proof_path = output_dir.join(PROOF_OUTPUT);
    let pv_path = output_dir.join(PUBLIC_VALUES_OUTPUT);
    std::fs::write(&proof_path, proof.bytes()).expect("failed to save proof");
    std::fs::write(&pv_path, &pv_bytes).expect("failed to save public values");
    println!("proof saved to {}", proof_path.display());
    println!("public values saved to {}", pv_path.display());
}
