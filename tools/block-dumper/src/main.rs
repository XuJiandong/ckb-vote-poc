use std::fs;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use anyhow::Context;
use ckb_gen_types::packed;
use ckb_vote_types::molecules::types::BlockVec;
use clap::Parser;
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

    /// Number of parallel download threads
    #[arg(short = 'n', long, default_value = "10")]
    threads: usize,

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

fn fetch_block_by_number(url: &str, block_number: u64) -> anyhow::Result<packed::Block> {
    rpc_call(
        url,
        "get_block_by_number",
        serde_json::json!([format!("0x{block_number:x}"), "0x0"]),
    )
    .with_context(|| format!("failed to fetch block number {block_number}"))
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let first_block = rpc_call(
        &args.url,
        "get_block",
        serde_json::json!([args.start_block_hash, "0x0"]),
    )?;

    let start_number: u64 = first_block.header().raw().number().into();
    let count = args.count as usize;
    let n_threads = args.threads.min(count);
    let url = Arc::new(args.url.clone());
    let done = Arc::new(AtomicUsize::new(0));

    eprintln!("Fetching {} blocks with {} threads...", count, n_threads);

    // Divide indices 0..count into n_threads chunks. Each thread fetches its
    // chunk sequentially, returning (index, block) pairs.
    let chunk_size = count.div_ceil(n_threads);
    let handles: Vec<_> = (0..n_threads)
        .map(|t| {
            let chunk_start = t * chunk_size;
            let chunk_end = (chunk_start + chunk_size).min(count);
            let url = Arc::clone(&url);
            let done = Arc::clone(&done);
            thread::spawn(move || -> anyhow::Result<Vec<(usize, packed::Block)>> {
                let mut results = Vec::with_capacity(chunk_end - chunk_start);
                for idx in chunk_start..chunk_end {
                    let block = fetch_block_by_number(&url, start_number + idx as u64)?;
                    results.push((idx, block));

                    let prev = done.fetch_add(1, Ordering::Relaxed);
                    let completed = prev + 1;
                    // Print at each 10% milestone (and always at 100%).
                    let prev_pct = prev * 10 / count;
                    let curr_pct = completed * 10 / count;
                    if curr_pct > prev_pct || completed == count {
                        eprintln!("  {}%  ({}/{})", curr_pct * 10, completed, count);
                    }
                }
                Ok(results)
            })
        })
        .collect();

    // Pre-fill with the first block already fetched, overwrite slot 0 from
    // the thread results to avoid a special-case after joining.
    let mut slots: Vec<Option<packed::Block>> = (0..count).map(|_| None).collect();
    slots[0] = Some(first_block);

    for handle in handles {
        let chunk = handle.join().expect("thread panicked")?;
        for (idx, block) in chunk {
            slots[idx] = Some(block);
        }
    }

    let blocks: Vec<packed::Block> = slots.into_iter().map(|s| s.unwrap()).collect();
    let block_vec: BlockVec = blocks.into();
    fs::write(&args.out, block_vec.as_slice())?;
    eprintln!("Written {} blocks to {}", count, args.out);
    Ok(())
}
