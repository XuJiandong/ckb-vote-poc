import { Command } from "commander";
import { createVote, consumeVote } from "../../vote.js";
import {
  buildSigner,
  configFromRpcUrl,
  DEFAULT_RPC_URL,
  die,
} from "../shared.js";

export function registerVote(program: Command): void {
  // cast vote
  program
    .command("vote")
    .description(
      "Cast a vote on a proposal. DAO deposits are auto-discovered from the signer address.",
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
    .requiredOption("--vote <choice>", "Vote choice: yes or no")
    .option("--rpc-url <url>", "CKB RPC endpoint", DEFAULT_RPC_URL)
    .action(async (opts) => {
      const choice = opts.vote.toLowerCase();
      if (choice !== "yes" && choice !== "no") {
        die(`--vote must be "yes" or "no", got: ${opts.vote}`);
      }

      try {
        const config = configFromRpcUrl(opts.rpcUrl);
        const signer = buildSigner(opts.privateKeyFile, config);

        const result = await createVote(signer, {
          proposalOutPoint: {
            txHash: opts.proposalTxHash,
            index: opts.proposalIndex ?? 0,
          },
          vote: choice as "yes" | "no",
          config,
        });

        console.log(`Vote (${choice}) submitted successfully.`);
        console.log(
          `  vote cell:  ${result.voteOutPoint.txHash}:${result.voteOutPoint.index}`,
        );
      } catch (err) {
        die(err);
      }
    });

  // recycle vote cell
  program
    .command("consume-vote")
    .description("Recycle a vote cell to reclaim the occupied CKB")
    .requiredOption(
      "--private-key-file <path>",
      "Path to file containing hex private key",
    )
    .requiredOption("--vote-tx-hash <hex>", "Transaction hash of the vote cell")
    .option("--vote-index <n>", "Output index of the vote cell", parseInt, 0)
    .option("--rpc-url <url>", "CKB RPC endpoint", DEFAULT_RPC_URL)
    .action(async (opts) => {
      try {
        const config = configFromRpcUrl(opts.rpcUrl);
        const signer = buildSigner(opts.privateKeyFile, config);

        const txHash = await consumeVote(
          signer,
          { txHash: opts.voteTxHash, index: opts.voteIndex ?? 0 },
          { config },
        );

        console.log("Vote cell consumed successfully.");
        console.log(`  tx hash: ${txHash}`);
      } catch (err) {
        die(err);
      }
    });
}
