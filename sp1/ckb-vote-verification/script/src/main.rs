use clap::Parser;
use sp1_sdk::{ProveRequest, Prover, ProverClient, ProvingKey, SP1Stdin, include_elf, utils};

const ELF: sp1_sdk::Elf = include_elf!("ckb-vote-verification-program");

const BLOCK_DATA: &[u8] = include_bytes!("../../../../crates/verification/tests/blocks.bin");

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long)]
    execute: bool,

    #[arg(long)]
    prove: bool,
}

#[tokio::main]
async fn main() {
    utils::setup_logger();

    let args = Args::parse();

    if args.execute == args.prove {
        eprintln!("Error: You must specify either --execute or --prove");
        std::process::exit(1);
    }

    let mut stdin = SP1Stdin::new();
    stdin.write_vec(BLOCK_DATA.to_vec());

    let client = ProverClient::builder().cpu().build().await;

    if args.execute {
        let (mut public_values, report) = client.execute(ELF, stdin).await.unwrap();
        let start_hash: [u8; 32] = public_values.read();
        let end_hash: [u8; 32] = public_values.read();
        let block_count: usize = public_values.read();
        let transaction_count: usize = public_values.read();

        println!("start block hash: 0x{}", hex::encode(start_hash));
        println!("end block hash:   0x{}", hex::encode(end_hash));
        println!("block count:       {}", block_count);
        println!("transaction count: {}", transaction_count);

        let blake2b_cycles = report.cycle_tracker.get("blake2b").unwrap();
        println!(
            "blake2b with {:.0} M instructions ",
            *blake2b_cycles as f64 / 1000.0 / 1000.0
        );
        println!(
            "executed program with {:.0} M instructions",
            report.total_instruction_count() as f64 / 1000.0 / 1000.0
        );
    } else {
        let pk = client.setup(ELF).await.unwrap();

        let mut proof = client.prove(&pk, stdin).core().await.unwrap();

        let start_hash: [u8; 32] = proof.public_values.read();
        let end_hash: [u8; 32] = proof.public_values.read();
        let block_count: usize = proof.public_values.read();
        let transaction_count: usize = proof.public_values.read();

        println!("start block hash: 0x{}", hex::encode(start_hash));
        println!("end block hash:   0x{}", hex::encode(end_hash));
        println!("block count:       {}", block_count);
        println!("transaction count: {}", transaction_count);

        client
            .verify(&proof, pk.verifying_key(), None)
            .expect("verification failed");

        println!("successfully generated and verified proof!");
    }
}
