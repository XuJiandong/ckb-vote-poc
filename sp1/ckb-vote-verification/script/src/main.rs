use ckb_vote_types::molecules::{
    blockchain,
    types::{Proposal, PublicValues},
};
use clap::Parser;
use molecule::prelude::{Builder, Entity};
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

    let cli = Args::parse();

    if cli.execute == cli.prove {
        eprintln!("Error: You must specify either --execute or --prove");
        std::process::exit(1);
    }

    let proposal = Proposal::new_builder()
        .vote_cell_code_hash(blockchain::Byte32::from([1u8; 32]))
        .vote_cell_hash_type(blockchain::Byte::new(0))
        .minimal_requirement(blockchain::Uint64::from(100u64.to_le_bytes()))
        .build();

    let guest_args = ckb_vote_testtool::generate_from_templates(proposal, BLOCK_DATA)
        .expect("generate_from_templates");

    let mut stdin = SP1Stdin::new();
    stdin.write_vec(guest_args.as_slice().to_vec());

    let client = ProverClient::builder().cpu().build().await;

    if cli.execute {
        #[allow(unused_mut)]
        let (mut public_values, report) = client.execute(ELF, stdin).await.unwrap();

        let pv_bytes = public_values.as_slice().to_vec();
        let pv = PublicValues::from_slice(&pv_bytes).expect("invalid PublicValues");
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
        assert_eq!(u8::from(pv.passed()), 1);
        #[cfg(not(feature = "profiling"))]
        let _ = (public_values, report);
    } else {
        let pk = client.setup(ELF).await.unwrap();

        let proof = client.prove(&pk, stdin).core().await.unwrap();

        let pv_bytes = proof.public_values.as_slice().to_vec();
        let pv = PublicValues::from_slice(&pv_bytes).expect("invalid PublicValues");
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

        client
            .verify(&proof, pk.verifying_key(), None)
            .expect("verification failed");
        assert_eq!(u8::from(pv.passed()), 1);
        println!("successfully generated and verified proof!");
    }
}
