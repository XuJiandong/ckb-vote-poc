import { Command } from "commander";
import { ccc } from "@ckb-ccc/shell";
import { ProposalCodec, VoteCodec } from "../../codec.js";
import { configFromRpcUrl, DEFAULT_RPC_URL, die } from "../shared.js";
import { blake160, buildClient } from "../../utils.js";

// ─── Raw RPC helpers ─────────────────────────────────────────────────────────

interface RpcResponse<T> {
  result?: T;
  error?: { code: number; message: string };
}

async function rpcCall<T>(
  rpcUrl: string,
  method: string,
  params: unknown[],
): Promise<T> {
  const res = await fetch(rpcUrl, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ id: 1, jsonrpc: "2.0", method, params }),
  });
  const data = (await res.json()) as RpcResponse<T>;
  if (data.error) throw new Error(`RPC error: ${data.error.message}`);
  if (data.result === undefined) throw new Error(`No result from ${method}`);
  return data.result;
}

// ─── RPC response types for get_cells ────────────────────────────────────────

interface RpcScript {
  code_hash: string;
  hash_type: string;
  args: string;
}

interface RpcCellOutput {
  capacity: string;
  lock: RpcScript;
  type?: RpcScript | null;
}

interface RpcCellObject {
  block_number: string;
  out_point: { tx_hash: string; index: string };
  output: RpcCellOutput;
  output_data?: string;
}

interface RpcGetCellsResult {
  last_cursor: string;
  objects: RpcCellObject[];
}

interface RpcTxStatus {
  block_number: string;
  block_hash: string;
}

interface RpcTransactionResult {
  tx_status: RpcTxStatus;
}

interface RpcBlock {
  header: { timestamp: string };
}

interface RpcLiveCell {
  status: string;
}

/**
 * Check whether a cell is still live (not consumed).
 */
async function isCellLive(
  rpcUrl: string,
  txHash: string,
  index: string,
): Promise<boolean> {
  try {
    const result = await rpcCall<RpcLiveCell>(rpcUrl, "get_live_cell", [
      { tx_hash: txHash, index },
      false,
    ]);
    return result.status === "live";
  } catch {
    return false;
  }
}

/**
 * Fetch cells matching the given type script via paginated get_cells RPC.
 *
 * - Pass `args: "0x"` with prefix search to match all cells of a code hash.
 * - Pass a specific args value for exact matching (e.g. votes for one proposal).
 * - Pass `maxResults` to stop collecting once that many cells are gathered.
 */
async function getAllCellsByType(
  rpcUrl: string,
  codeHash: string,
  hashType: string,
  args: string,
  order: "asc" | "desc" = "asc",
  maxResults?: number,
): Promise<RpcCellObject[]> {
  const allCells: RpcCellObject[] = [];
  let cursor: string | null = null;
  const pageSize = maxResults ? Math.min(maxResults, 100) : 100;
  const pageLimitHex = "0x" + pageSize.toString(16);
  const searchMode = args === "0x" ? "prefix" : "exact";

  while (true) {
    const result: RpcGetCellsResult = await rpcCall<RpcGetCellsResult>(
      rpcUrl,
      "get_cells",
      [
        {
          script: { code_hash: codeHash, hash_type: hashType, args },
          script_type: "type",
          script_search_mode: searchMode,
          with_data: true,
        },
        order,
        pageLimitHex,
        cursor,
      ],
    );

    allCells.push(...result.objects);

    if (result.objects.length < pageSize) break;
    if (maxResults !== undefined && allCells.length >= maxResults) break;
    cursor = result.last_cursor;
  }

  return maxResults !== undefined ? allCells.slice(0, maxResults) : allCells;
}

/**
 * Fetch block number and timestamp for a transaction.
 */
async function getBlockTimestamp(
  rpcUrl: string,
  txHash: string,
): Promise<{ blockNumber: string; timestamp: string } | null> {
  try {
    const txResult = await rpcCall<RpcTransactionResult>(
      rpcUrl,
      "get_transaction",
      [txHash],
    );
    const blockNumber = parseInt(txResult.tx_status.block_number, 16);
    const block = await rpcCall<RpcBlock>(rpcUrl, "get_block_by_number", [
      txResult.tx_status.block_number,
      "0x2",
    ]);
    const timestampMs = parseInt(block.header.timestamp, 16);
    const d = new Date(timestampMs);
    const ts = d
      .toLocaleString("sv-SE", { timeZone: "Asia/Shanghai" })
      .replace("T", " ");
    return {
      blockNumber: blockNumber.toString(),
      timestamp: ts + " +08:00",
    };
  } catch {
    return null;
  }
}

/** Convert an RpcCellObject into a ccc.Cell for decoding. */
function rpcCellToCccCell(obj: RpcCellObject): ccc.Cell {
  return ccc.Cell.from({
    outPoint: ccc.OutPoint.from({
      txHash: obj.out_point.tx_hash,
      index: parseInt(obj.out_point.index, 16),
    }),
    cellOutput: ccc.CellOutput.from({
      capacity: BigInt(obj.output.capacity),
      lock: ccc.Script.from({
        codeHash: obj.output.lock.code_hash,
        hashType: obj.output.lock.hash_type,
        args: obj.output.lock.args,
      }),
      type: obj.output.type
        ? ccc.Script.from({
            codeHash: obj.output.type.code_hash,
            hashType: obj.output.type.hash_type,
            args: obj.output.type.args,
          })
        : undefined,
    }),
    outputData: obj.output_data ?? "0x",
  });
}

// ─── Query helpers ───────────────────────────────────────────────────────────

interface ProposalInfo {
  cell: ccc.Cell;
  decoded: ccc.mol.DecodedType<typeof ProposalCodec>;
}

interface VoteInfo {
  cell: ccc.Cell;
  decoded: ccc.mol.DecodedType<typeof VoteCodec>;
}

async function collectProposals(
  rpcUrl: string,
  config: ReturnType<typeof configFromRpcUrl>,
  count: number,
): Promise<ProposalInfo[]> {
  // desc order → newest cells come first; stop once we have `count` proposals.
  const rawCells = await getAllCellsByType(
    rpcUrl,
    config.proposalTypeScript.codeHash,
    config.proposalTypeScript.hashType,
    "0x",
    "desc",
    count,
  );

  const proposals: ProposalInfo[] = [];
  for (const raw of rawCells) {
    if (raw.output_data && raw.output_data !== "0x") {
      try {
        const cell = rpcCellToCccCell(raw);
        const decoded = ProposalCodec.decode(raw.output_data);
        proposals.push({ cell, decoded });
      } catch {
        // skip malformed
      }
    }
  }
  return proposals;
}

async function collectVotesForProposal(
  rpcUrl: string,
  config: ReturnType<typeof configFromRpcUrl>,
  voteTypeArgs: string,
): Promise<VoteInfo[]> {
  const rawCells = await getAllCellsByType(
    rpcUrl,
    config.voteTypeScript.codeHash,
    config.voteTypeScript.hashType,
    voteTypeArgs,
    "asc",
  );

  const votes: VoteInfo[] = [];
  for (const raw of rawCells) {
    if (raw.output_data && raw.output_data !== "0x") {
      try {
        const cell = rpcCellToCccCell(raw);
        const decoded = VoteCodec.decode(raw.output_data);
        votes.push({ cell, decoded });
      } catch {
        // skip malformed
      }
    }
  }
  return votes;
}

// ─── Command ─────────────────────────────────────────────────────────────────

export function registerQuery(program: Command): void {
  program
    .command("query")
    .description(
      "Query all proposal cells and their associated vote cells on chain",
    )
    .option("--rpc-url <url>", "CKB RPC endpoint", DEFAULT_RPC_URL)
    .option(
      "--count <n>",
      "Number of latest proposals to show, sorted by time",
      parseInt,
      3,
    )
    .action(async (opts) => {
      try {
        const count = opts.count as number;
        const config = configFromRpcUrl(opts.rpcUrl as string);
        const client = buildClient(config.ckbRpcUrl, config.knownScripts);

        // Fetch the `count` most-recent proposals (desc order, stops early).
        const proposals = await collectProposals(
          config.ckbRpcUrl,
          config,
          count,
        );

        // Cache block info to avoid duplicate RPC calls
        const blockInfoCache = new Map<
          string,
          {
            blockNumber: string;
            timestamp: string;
            blockNumberNum: number;
          } | null
        >();

        const getBlockInfo = async (txHash: string) => {
          if (!blockInfoCache.has(txHash)) {
            const info = await getBlockTimestamp(config.ckbRpcUrl, txHash);
            if (info) {
              blockInfoCache.set(txHash, {
                ...info,
                blockNumberNum: parseInt(info.blockNumber, 10),
              });
            } else {
              blockInfoCache.set(txHash, null);
            }
          }
          return blockInfoCache.get(txHash)!;
        };

        // Resolve block info for all proposals in parallel, then sort newest-first.
        const proposalsWithBlock = await Promise.all(
          proposals.map(async (p) => {
            const outPoint = p.cell.outPoint!;
            const info = await getBlockInfo(outPoint.txHash);
            return {
              ...p,
              outStr: `${outPoint.txHash}:${outPoint.index}`,
              blockNumberNum: info?.blockNumberNum ?? 0,
            };
          }),
        );
        proposalsWithBlock.sort((a, b) => b.blockNumberNum - a.blockNumberNum);

        // Check live status for all proposals in parallel.
        const liveStatusCache = new Map<string, boolean>();
        await Promise.all(
          proposalsWithBlock.map(async (p) => {
            const outPoint = p.cell.outPoint!;
            const live = await isCellLive(
              config.ckbRpcUrl,
              outPoint.txHash,
              "0x" + outPoint.index.toString(16),
            );
            liveStatusCache.set(p.outStr, live);
          }),
        );

        // Fetch votes for each proposal in parallel (on-demand, by exact args).
        const votesPerProposal = await Promise.all(
          proposalsWithBlock.map((p) => {
            const voteTypeArgs = blake160(p.cell.cellOutput.type!.toBytes());
            return collectVotesForProposal(
              config.ckbRpcUrl,
              config,
              voteTypeArgs,
            );
          }),
        );

        // Print results grouped by proposal.
        console.log(
          `=== CKB Vote Query Results (latest ${proposalsWithBlock.length}) ===\n`,
        );

        if (proposalsWithBlock.length === 0) {
          console.log("No proposal cells found on chain.");
        }

        for (let pi = 0; pi < proposalsWithBlock.length; pi++) {
          const {
            cell,
            decoded: proposalData,
            outStr,
          } = proposalsWithBlock[pi];
          const outPoint = cell.outPoint!;
          const blockInfo = blockInfoCache.get(outPoint.txHash);
          const live = liveStatusCache.get(outStr) ?? false;
          const status = live ? "ACTIVE" : "CONSUMED";
          const relatedVotes = votesPerProposal[pi];

          console.log(
            `── Proposal ${pi + 1}/${proposalsWithBlock.length} [${status}] ──`,
          );
          console.log(`  cell:  ${outPoint.txHash}:${outPoint.index} (output)`);
          if (blockInfo) {
            console.log(`  block: #${blockInfo.blockNumber}`);
            console.log(`  time:  ${blockInfo.timestamp}`);
          }

          const descBytes = ccc.bytesFrom(proposalData.description);
          const description = new TextDecoder().decode(descBytes);
          console.log(`  description: ${description}`);
          console.log(`  duration: ${proposalData.duration} blocks`);
          console.log(
            `  amount: ${(Number(proposalData.amount) / 1e8).toFixed(2)} CKB`,
          );

          try {
            const receiverAddr = ccc.Address.fromScript(
              proposalData.receiver,
              client,
            );
            console.log(`  receiver: ${receiverAddr}`);
          } catch {
            console.log(
              `  receiver: (script) codeHash=${proposalData.receiver.codeHash}, args=${proposalData.receiver.args}`,
            );
          }
          console.log(
            `  minimal_requirement: ${Number(proposalData.minimalRequirement).toFixed(0)} CKB`,
          );

          if (relatedVotes.length > 0) {
            console.log(`  votes (${relatedVotes.length}):`);
            for (let vi = 0; vi < relatedVotes.length; vi++) {
              const { cell: voteCell, decoded: voteData } = relatedVotes[vi];
              const vOutPoint = voteCell.outPoint!;
              const vBlockInfo = await getBlockInfo(vOutPoint.txHash);

              const voteText = voteData.vote === 1 ? "YES" : "NO";
              const voteAmount = (Number(voteData.amount) / 1e8).toFixed(2);

              let line = `    ${vi + 1}. ${vOutPoint.txHash}:${vOutPoint.index} (output)`;
              line += ` - ${voteText}`;
              line += `, ${voteAmount} CKB`;
              if (vBlockInfo) {
                line += `, block #${vBlockInfo.blockNumber}`;
              }
              console.log(line);
            }
          } else {
            console.log(`  votes: (none)`);
          }
          console.log();
        }

        // Summary
        const totalVoteCount = votesPerProposal.reduce(
          (sum, vs) => sum + vs.length,
          0,
        );
        console.log(
          `=== Summary: ${proposalsWithBlock.length} proposal(s), ${totalVoteCount} vote(s) ===`,
        );
      } catch (err) {
        die(err);
      }
    });
}
