import { Command } from "commander";
import { consumeProposal } from "../../proposal.js";
import {
  buildSigner,
  configFromRpcUrl,
  DEFAULT_RPC_URL,
  die,
  readFileAsHex,
} from "../shared.js";

export function registerConsumeProposal(program: Command): void {
  program
    .command("consume-proposal")
    .description(
      "Consume a passed proposal cell by providing an SP1 PLONK proof",
    )
    .requiredOption(
      "--private-key-file <path>",
      "Path to file containing hex private key",
    )
    .requiredOption(
      "--proposal-tx-hash <hex>",
      "Transaction hash of the proposal cell",
    )
    .option(
      "--proposal-index <n>",
      "Output index of the proposal cell",
      parseInt,
      0,
    )
    .requiredOption(
      "--proof <path>",
      "Path to the SP1 PLONK proof binary file (e.g. proof-plonk.bin)",
    )
    .requiredOption(
      "--public-values <path>",
      "Path to the SP1 public values binary file (e.g. public-values.bin)",
    )
    .requiredOption(
      "--start-block-hash <hex>",
      "Hash of the block containing the proposal cell (header_deps[0])",
    )
    .requiredOption(
      "--end-block-hash <hex>",
      "Hash of the end block (start + duration, header_deps[1])",
    )
    .option("--rpc-url <url>", "CKB RPC endpoint", DEFAULT_RPC_URL)
    .action(async (opts) => {
      try {
        const config = configFromRpcUrl(opts.rpcUrl);
        const signer = buildSigner(opts.privateKeyFile, config);

        const proofHex = readFileAsHex(opts.proof);
        const publicValuesHex = readFileAsHex(opts.publicValues);

        const result = await consumeProposal(signer, {
          proposalOutPoint: {
            txHash: opts.proposalTxHash,
            index: opts.proposalIndex ?? 0,
          },
          proof: proofHex,
          publicValues: publicValuesHex,
          startBlockHash: opts.startBlockHash,
          endBlockHash: opts.endBlockHash,
          config,
        });

        console.log("Proposal consumed successfully.");
        console.log(`  tx hash:      ${result.txHash}`);
        console.log(`  output index: ${result.outputIndex}`);
      } catch (err) {
        die(err);
      }
    });
}
