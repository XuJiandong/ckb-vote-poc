use sp1_sdk::{
    ProveRequest, Prover, ProverClient, ProvingKey, SP1Stdin, include_elf, network::NetworkMode,
    utils,
};

const ELF: sp1_sdk::Elf = include_elf!("ckb-vote-verification-program");

const BLOCK_DATA: &[u8] = include_bytes!("../../../../crates/verification/tests/blocks.bin");

const PROOF_OUTPUT: &str = "proof-plonk.bin";

#[tokio::main]
async fn main() {
    utils::setup_logger();

    let mut stdin = SP1Stdin::new();
    stdin.write_vec(BLOCK_DATA.to_vec());

    // NETWORK_PRIVATE_KEY env var must be set to your requester account's private key.
    let client = ProverClient::builder()
        .network_for(NetworkMode::Mainnet)
        .build()
        .await;

    let pk = client.setup(ELF).await.unwrap();

    let mut proof = client.prove(&pk, stdin).plonk().await.unwrap();

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

    proof.save(PROOF_OUTPUT).expect("failed to save proof");
    println!("proof saved to {PROOF_OUTPUT}");
}
