use ckb_vote_types::molecules::{
    blockchain,
    types::{BlockVecReader, Proposal},
};
use criterion::{Criterion, criterion_group, criterion_main};
use molecule::prelude::{Builder, Entity, Reader};

const BLOCK_DATA: &[u8] = include_bytes!("../tests/blocks.bin");

fn bench_count_vote(c: &mut Criterion) {
    let proposal = Proposal::new_builder()
        .vote_cell_code_hash(blockchain::Byte32::from([1u8; 32]))
        .vote_cell_hash_type(blockchain::Byte::new(0))
        .minimal_requirement(blockchain::Uint64::from(0u64.to_le_bytes()))
        .build();

    let args = ckb_vote_testtool::generate_from_templates(proposal, BLOCK_DATA)
        .expect("generate_from_templates");

    let args_reader = args.as_reader();
    // Collect blocks bytes so the benchmark owns them and can lend references each iteration.
    let blocks_bytes: Vec<u8> = args_reader.blocks().raw_data().to_vec();
    let proposal_script = args_reader.proposal_script().to_entity();

    let num_blocks = BlockVecReader::new_unchecked(&blocks_bytes).len();
    println!("Benchmarking count_vote with {num_blocks} blocks");

    c.bench_function("count_vote", |b| {
        b.iter(|| {
            // BlockVecReader::new_unchecked is a zero-cost slice wrap; validation
            // was already done by generate_from_templates above.
            let blocks = BlockVecReader::new_unchecked(&blocks_bytes);
            let result = ckb_vote_verification::count_vote(blocks, proposal_script.clone());
            assert!(result.passed, "expected passed = true");
            result
        });
    });
}

criterion_group!(benches, bench_count_vote);
criterion_main!(benches);
