import { Command } from "commander";
import { existsSync, readFileSync, readdirSync } from "node:fs";
import { join, resolve } from "node:path";
import { configFromRpcUrl, DEFAULT_RPC_URL } from "../shared.js";
import type { NetworkConfig, ScriptInfo } from "../../config.js";

function ok(msg: string): void {
  console.log(`  ok   ${msg}`);
}

function fail(msg: string): void {
  console.log(`  FAIL ${msg}`);
}

interface MigrationRecipe {
  name: string;
  tx_hash: string;
  index: number;
  type_id?: string;
}

interface Migration {
  cell_recipes: MigrationRecipe[];
}

function checkScriptVsDeployment(
  name: string,
  info: ScriptInfo,
  deploymentDir: string,
  subdir: string,
): boolean {
  const migDir = join(deploymentDir, subdir, "migrations");
  if (!existsSync(migDir)) {
    fail(`${name}: migration directory not found: ${migDir}`);
    return false;
  }

  const files = readdirSync(migDir)
    .filter((f) => f.endsWith(".json"))
    .sort();
  if (files.length === 0) {
    fail(`${name}: no migration JSON files in ${migDir}`);
    return false;
  }

  const migFile = join(migDir, files[files.length - 1]);
  let mig: Migration;
  try {
    mig = JSON.parse(readFileSync(migFile, "utf8")) as Migration;
  } catch {
    fail(`${name}: failed to parse ${migFile}`);
    return false;
  }

  const recipe = mig.cell_recipes?.[0];
  if (!recipe) {
    fail(`${name}: no cell_recipes in ${migFile}`);
    return false;
  }

  let allOk = true;

  if (recipe.type_id !== info.codeHash) {
    fail(
      `${name} codeHash: config=${info.codeHash}, deployment=${recipe.type_id ?? "(missing)"}`,
    );
    allOk = false;
  }

  if (recipe.tx_hash !== info.outPoint.txHash) {
    fail(
      `${name} outPoint.txHash: config=${info.outPoint.txHash}, deployment=${recipe.tx_hash}`,
    );
    allOk = false;
  }

  if (recipe.index !== info.outPoint.index) {
    fail(
      `${name} outPoint.index: config=${info.outPoint.index}, deployment=${recipe.index}`,
    );
    allOk = false;
  }

  if (allOk) {
    ok(
      `${name}: ${info.outPoint.txHash}:${info.outPoint.index} codeHash=${info.codeHash}`,
    );
  }
  return allOk;
}

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

interface LiveCellResult {
  status: string;
}

async function checkScriptCellAlive(
  name: string,
  outPoint: { txHash: string; index: number },
  rpcUrl: string,
): Promise<boolean> {
  const hexIndex = `0x${outPoint.index.toString(16)}`;
  let result: LiveCellResult;
  try {
    result = await rpcCall<LiveCellResult>(rpcUrl, "get_live_cell", [
      { tx_hash: outPoint.txHash, index: hexIndex },
      false,
    ]);
  } catch (e) {
    fail(
      `${name} live cell: RPC error: ${e instanceof Error ? e.message : String(e)}`,
    );
    return false;
  }
  if (result.status === "live") {
    ok(`${name} live cell: ${outPoint.txHash}:${outPoint.index} is alive`);
    return true;
  }
  fail(
    `${name} live cell: ${outPoint.txHash}:${outPoint.index} status=${result.status}`,
  );
  return false;
}

interface RpcTransaction {
  hash: string;
  outputs: unknown[];
}

interface RpcBlock {
  transactions: RpcTransaction[];
}

async function fetchGenesisBlock(rpcUrl: string): Promise<RpcBlock> {
  return rpcCall<RpcBlock>(rpcUrl, "get_block_by_number", ["0x0", "0x2"]);
}

async function checkKnownScriptCellDeps(
  config: NetworkConfig,
): Promise<boolean> {
  if (!config.knownScripts || Object.keys(config.knownScripts).length === 0) {
    console.log("  (no knownScripts configured, skipping)");
    return true;
  }

  let genesisBlock: RpcBlock;
  try {
    genesisBlock = await fetchGenesisBlock(config.ckbRpcUrl);
  } catch (e) {
    fail(
      `Failed to fetch genesis block: ${e instanceof Error ? e.message : String(e)}`,
    );
    return false;
  }

  const genesisTxMap = new Map<string, RpcTransaction>();
  for (const tx of genesisBlock.transactions) {
    genesisTxMap.set(tx.hash, tx);
  }

  let allOk = true;

  for (const [scriptName, scriptInfo] of Object.entries(config.knownScripts)) {
    for (const { cellDep } of scriptInfo.cellDeps) {
      const { txHash, index } = cellDep.outPoint;
      const genesisTx = genesisTxMap.get(txHash);
      if (!genesisTx) {
        fail(`${scriptName} cellDep: tx ${txHash} not found in genesis block`);
        allOk = false;
        continue;
      }
      if (index >= genesisTx.outputs.length) {
        fail(
          `${scriptName} cellDep: ${txHash}:${index} - index out of range (genesis tx has ${genesisTx.outputs.length} outputs)`,
        );
        allOk = false;
        continue;
      }
      ok(`${scriptName} cellDep: ${txHash}:${index} is in genesis block`);
    }
  }

  return allOk;
}

export function registerCheck(program: Command): void {
  program
    .command("check")
    .description(
      "Verify SDK configuration against deployment files and chain state",
    )
    .option("--rpc-url <url>", "CKB RPC endpoint", DEFAULT_RPC_URL)
    .option(
      "--deployment-dir <path>",
      "Path to deployment/devnet directory",
      "../deployment/devnet",
    )
    .option(
      "--vk-file <path>",
      "Path to SP1 verifying key hash file",
      "../sp1/ckb-vote-verification/verifying-key.txt",
    )
    .action(async (opts) => {
      const config = configFromRpcUrl(opts.rpcUrl as string);
      const deploymentDir = resolve(opts.deploymentDir as string);
      const vkFile = resolve(opts.vkFile as string);
      let allOk = true;

      // 1. Script configs vs deployment migration files
      console.log("Checking script configurations against deployment files...");
      const scriptChecks: Array<{
        name: string;
        dir: string;
        info: ScriptInfo;
      }> = [
        {
          name: "alwaysSuccess",
          dir: "always-success",
          info: config.alwaysSuccess,
        },
        {
          name: "proposalTypeScript",
          dir: "proposal-type-script",
          info: config.proposalTypeScript,
        },
        {
          name: "voteTypeScript",
          dir: "vote-type-script",
          info: config.voteTypeScript,
        },
      ];
      for (const { name, dir, info } of scriptChecks) {
        if (!checkScriptVsDeployment(name, info, deploymentDir, dir)) {
          allOk = false;
        }
        if (
          !(await checkScriptCellAlive(name, info.outPoint, config.ckbRpcUrl))
        ) {
          allOk = false;
        }
      }

      // 2. SP1 verifying key hash
      console.log("\nChecking SP1 verifying key hash...");
      if (!existsSync(vkFile)) {
        fail(`VK file not found: ${vkFile}`);
        allOk = false;
      } else {
        const raw = readFileSync(vkFile, "utf8").trim();
        const fileHash = raw.startsWith("0x") ? raw : `0x${raw}`;
        if (
          fileHash.toLowerCase() === config.sp1VerifyingKeyHash.toLowerCase()
        ) {
          ok(`sp1VerifyingKeyHash: ${config.sp1VerifyingKeyHash}`);
        } else {
          fail(
            `sp1VerifyingKeyHash: config=${config.sp1VerifyingKeyHash}, vk-file=${fileHash}`,
          );
          allOk = false;
        }
      }

      // 3. knownScripts cellDeps in genesis block
      console.log("\nChecking knownScripts cellDeps are in genesis block...");
      if (!(await checkKnownScriptCellDeps(config))) {
        allOk = false;
      }

      console.log();
      if (allOk) {
        console.log("All checks passed.");
        process.exit(0);
      } else {
        console.log("Some checks failed.");
        process.exit(1);
      }
    });
}
