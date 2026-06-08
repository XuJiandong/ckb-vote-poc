import { Command } from "commander";
import { createProposal } from "../../proposal.js";
import {
  buildSigner,
  configFromRpcUrl,
  DEFAULT_RPC_URL,
  die,
} from "../shared.js";

export function registerCreateProposal(program: Command): void {
  program
    .command("create-proposal")
    .description("Create a new proposal cell on-chain")
    .requiredOption(
      "--private-key-file <path>",
      "Path to file containing hex private key",
    )
    .requiredOption(
      "--duration <blocks>",
      "Voting duration in blocks",
      parseInt,
    )
    .requiredOption(
      "--description <text>",
      "Plain-text description of the proposal",
    )
    .option(
      "--receiver <address>",
      "CKB address to receive funds if proposal passes (defaults to signer's address)",
    )
    .option(
      "--amount <ckb>",
      "Amount (in CKB) to transfer when proposal passes (defaults to 0)",
      parseFloat,
    )
    .option(
      "--minimal-requirement <ckb>",
      "Minimum total CKB vote weight required for proposal to pass (defaults to 0)",
      parseFloat,
    )
    .option("--rpc-url <url>", "CKB RPC endpoint", DEFAULT_RPC_URL)
    .action(async (opts) => {
      try {
        const config = configFromRpcUrl(opts.rpcUrl);
        const signer = buildSigner(opts.privateKeyFile, config);

        const amountShannon =
          opts.amount !== undefined
            ? BigInt(Math.round(opts.amount * 1e8))
            : 0n;
        const minReqShannon =
          opts.minimalRequirement !== undefined
            ? BigInt(Math.round(opts.minimalRequirement * 1e8))
            : 0n;

        const result = await createProposal(signer, {
          duration: opts.duration,
          description: opts.description,
          receiver: opts.receiver,
          amount: amountShannon,
          minimalRequirement: minReqShannon,
          config,
        });

        console.log("Proposal created successfully.");
        console.log(`  tx hash:      ${result.txHash}`);
        console.log(
          `  outpoint:     ${result.proposalOutPoint.txHash}:${result.proposalOutPoint.index}`,
        );
      } catch (err) {
        die(err);
      }
    });
}
