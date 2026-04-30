use std::fs;

use anyhow::Context;
use clap::Parser;
use ckb_gen_types::packed;
use ckb_vote_types::molecules::types::BlockVec;
use molecule::prelude::{Builder, Entity, Reader};
use serde::Deserialize;

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "https://mainnet.ckb.dev")]
    url: String,

    #[arg(long)]
    start_block_hash: String,

    #[arg(long)]
    count: u64,

    #[arg(long)]
    out: String,
}

#[derive(Deserialize)]
struct RpcResponse {
    result: Option<String>,
}

#[derive(serde::Serialize)]
struct RpcRequest {
    id: u64,
    jsonrpc: String,
    method: String,
    params: serde_json::Value,
}

fn parse_block(bytes: &[u8]) -> anyhow::Result<packed::Block> {
    if let Ok(block) = packed::Block::from_slice(bytes) {
        return Ok(block);
    }
    let block_v1 = packed::BlockV1::from_slice(bytes)?;
    let reader = block_v1.as_reader();
    let block = packed::Block::new_builder()
        .header(reader.header().to_entity())
        .uncles(reader.uncles().to_entity())
        .transactions(reader.transactions().to_entity())
        .proposals(reader.proposals().to_entity())
        .build();
    Ok(block)
}

fn rpc_call(url: &str, method: &str, params: serde_json::Value) -> anyhow::Result<packed::Block> {
    let client = reqwest::blocking::Client::new();
    let req = RpcRequest {
        id: 1,
        jsonrpc: "2.0".into(),
        method: method.into(),
        params,
    };
    let resp: RpcResponse = client.post(url).json(&req).send()?.json()?;
    let result = resp.result.context("RPC returned null result")?;
    let hex_str = result
        .strip_prefix("0x")
        .context("result does not start with 0x")?;
    let bytes = hex::decode(hex_str)?;
    parse_block(&bytes)
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let first_block = rpc_call(
        &args.url,
        "get_block",
        serde_json::json!([args.start_block_hash, "0x0"]),
    )?;

    let start_number: u64 = first_block.header().raw().number().into();
    let mut blocks: Vec<packed::Block> = Vec::with_capacity(args.count as usize);
    blocks.push(first_block);

    for i in 1..args.count {
        let block_number = start_number + i;
        let block = rpc_call(
            &args.url,
            "get_block_by_number",
            serde_json::json!([format!("0x{block_number:x}"), "0x0"]),
        )
        .with_context(|| format!("failed to fetch block number {block_number}"))?;
        blocks.push(block);
    }

    let block_vec: BlockVec = blocks.into();
    fs::write(&args.out, block_vec.as_slice())?;
    eprintln!("Written {} blocks to {}", args.count, args.out);
    Ok(())
}
