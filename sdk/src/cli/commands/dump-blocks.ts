import { Command } from "commander";
import { writeFileSync } from "node:fs";
import { ccc } from "@ckb-ccc/shell";
import { DEFAULT_RPC_URL, die } from "../shared.js";
import { buildClient } from "../../utils.js";

const mol = ccc.mol;

// Codec that passes raw bytes through as a molecule DynVec item.
const RawField = mol.Codec.from<ccc.Bytes, ccc.Bytes>({
  encode: (bytes) => bytes,
  decode: (bytes) => ccc.bytesFrom(bytes),
});

// BlockVec: molecule DynVec of raw block bytes
const BlockVecCodec = mol.dynItemVec(RawField);

/**
 * Look up the block number of the transaction that contains the proposal output cell.
 */
async function findStartBlockNumber(
  client: ccc.ClientPublicTestnet,
  txHash: string,
): Promise<bigint> {
  const result = (await client.requestor.request("get_transaction", [
    txHash,
  ])) as { tx_status: { block_number: string | null } } | null;

  if (!result || result.tx_status.block_number == null) {
    throw new Error(
      `Transaction ${txHash} not found or not yet committed on chain.`,
    );
  }

  return BigInt(result.tx_status.block_number);
}

/**
 * Fetch a block's raw molecule bytes using get_block_by_number with verbosity 0x0.
 */
async function fetchBlockBytes(
  client: ccc.ClientPublicTestnet,
  blockNumber: bigint,
): Promise<ccc.Bytes> {
  const hexNumber = `0x${blockNumber.toString(16)}`;
  const result = (await client.requestor.request("get_block_by_number", [
    hexNumber,
    "0x0",
  ])) as string;
  if (!result.startsWith("0x")) {
    throw new Error(`Unexpected result format for block ${blockNumber}`);
  }
  return ccc.bytesFrom(result);
}

export function registerDumpBlocks(program: Command): void {
  program
    .command("dump-blocks")
    .description(
      "Fetch a consecutive range of CKB blocks, starting from the block where " +
        "the proposal type script first appears, and write them as a molecule BlockVec file",
    )
    .requiredOption(
      "--proposal-tx-hash <hash>",
      "transaction hash of the proposal output cell (used to locate the start block)",
    )
    .option(
      "--proposal-index <index>",
      "output index of the proposal cell within that transaction",
      (v) => parseInt(v, 10),
      0,
    )
    .requiredOption("--count <n>", "Number of blocks to dump", (v) =>
      parseInt(v, 10),
    )
    .requiredOption("--out <path>", "Output file path")
    .option("--rpc-url <url>", "CKB RPC endpoint", DEFAULT_RPC_URL)
    .option(
      "--concurrency <n>",
      "Number of parallel fetch operations",
      (v) => parseInt(v, 10),
      10,
    )
    .action(async (opts) => {
      const rpcUrl = opts.rpcUrl as string;
      const count = opts.count as number;
      const concurrency = Math.min(opts.concurrency as number, count);
      const txHash = opts.proposalTxHash as string;

      const client = buildClient(rpcUrl);

      try {
        process.stderr.write(
          `Finding start block for proposal tx ${txHash}...\n`,
        );
        const startNumber = await findStartBlockNumber(client, txHash);
        process.stderr.write(
          `Starting at block #${startNumber}, fetching ${count} blocks with concurrency ${concurrency}...\n`,
        );

        const blocks = new Array<ccc.Bytes>(count);
        let done = 0;
        let nextIdx = 0;

        async function worker(): Promise<void> {
          while (true) {
            const idx = nextIdx++;
            if (idx >= count) break;
            blocks[idx] = await fetchBlockBytes(
              client,
              startNumber + BigInt(idx),
            );
            const prev = done++;
            const prevPct = Math.floor((prev / count) * 10);
            const currPct = Math.floor(((prev + 1) / count) * 10);
            if (currPct > prevPct || prev + 1 === count) {
              process.stderr.write(
                `  ${currPct * 10}%  (${prev + 1}/${count})\n`,
              );
            }
          }
        }

        await Promise.all(Array.from({ length: concurrency }, () => worker()));

        writeFileSync(opts.out as string, BlockVecCodec.encode(blocks));
        process.stderr.write(
          `Written ${count} blocks to ${opts.out as string}\n`,
        );
      } catch (err) {
        die(err);
      }
    });
}
