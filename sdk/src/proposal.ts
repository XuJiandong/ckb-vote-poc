import { ccc } from "@ckb-ccc/shell";
import { DEVNET_CONFIG, type NetworkConfig } from "./config.js";
import {
  ProposalCodec,
  ProposalWitnessCodec,
  PublicValuesCodec,
} from "./codec.js";
import {
  buildClient,
  buildProposalTypeScriptArgs,
  cellDepFromInfo,
  cellDepFromOutPoint,
  getRequiredCell,
  getSignerLock,
  hashTypeToByte,
  mergeConfig,
  scriptFromInfo,
} from "./utils.js";

// ─── Create Proposal ─────────────────────────────────────────────────────────

export interface CreateProposalParams {
  duration: number;
  description: string;
  /** CKB address string; defaults to signer's own address */
  receiver?: string;
  /** amount in shannon; defaults to 0n */
  amount?: bigint;
  /** minimal_requirement in shannon; defaults to 0n */
  minimalRequirement?: bigint;
  config?: Partial<NetworkConfig>;
}

export interface CreateProposalResult {
  txHash: string;
  /** Outpoint of the created proposal cell */
  proposalOutPoint: { txHash: string; index: number };
}

/**
 * Create a proposal cell on-chain.
 *
 * The proposal type script args are computed from the first input's outpoint
 * (Type ID mechanism), so the final args are only known after inputs are
 * collected. The function re-sets the args in two passes.
 */
export async function createProposal(
  signer: ccc.Signer,
  params: CreateProposalParams,
): Promise<CreateProposalResult> {
  const config = mergeConfig(DEVNET_CONFIG, params.config);
  const client = signer.client;

  const amount = params.amount ?? 0n;
  const minimalRequirement = params.minimalRequirement ?? 0n;

  // Resolve receiver lock script
  let receiverLock: ccc.Script;
  if (params.receiver) {
    const addr = await ccc.Address.fromString(params.receiver, client);
    receiverLock = addr.script;
  } else {
    receiverLock = await getSignerLock(signer);
  }

  const alwaysSuccessLock = scriptFromInfo(config.alwaysSuccess);
  const proposalTypeMeta = config.proposalTypeScript;

  // Placeholder args (52 bytes zeros) for capacity estimation
  const placeholderArgs = ("0x" + "00".repeat(52)) as ccc.Hex;
  const placeholderTypeScript = ccc.Script.from({
    codeHash: proposalTypeMeta.codeHash,
    hashType: proposalTypeMeta.hashType,
    args: placeholderArgs,
  });

  // Build proposal cell data (args don't affect data size)
  const proposalData = buildProposalData(
    params,
    config,
    receiverLock,
    amount,
    minimalRequirement,
  );
  const proposalDataHex = ccc.hexFrom(proposalData);

  // Build the output with auto-calculated minimum capacity
  const proposalOutput = ccc.CellOutput.from(
    {
      capacity: 0n,
      lock: alwaysSuccessLock,
      type: placeholderTypeScript,
    },
    proposalDataHex,
  );

  const tx = ccc.Transaction.from({
    outputs: [proposalOutput],
    outputsData: [proposalDataHex],
  });

  // Add required cell deps
  tx.addCellDeps(cellDepFromInfo(config.alwaysSuccess));
  tx.addCellDeps(cellDepFromInfo(config.proposalTypeScript));

  // Collect inputs to cover capacity
  await tx.completeInputsByCapacity(signer);

  // Now we know the first input — compute the real type ID args
  if (tx.inputs.length === 0) {
    throw new Error("No inputs collected");
  }
  const realArgs = buildProposalTypeScriptArgs(
    tx.inputs[0],
    0, // proposal cell is at output index 0
    config.sp1VerifyingKeyHash,
  );

  tx.outputs[0].type = ccc.Script.from({
    codeHash: proposalTypeMeta.codeHash,
    hashType: proposalTypeMeta.hashType,
    args: realArgs,
  });

  await tx.completeFeeBy(signer, config.feeRate);
  const txHash = await signer.sendTransaction(tx);

  return {
    txHash,
    proposalOutPoint: { txHash, index: 0 },
  };
}

function buildProposalData(
  params: CreateProposalParams,
  config: NetworkConfig,
  receiverLock: ccc.Script,
  amount: bigint,
  minimalRequirement: bigint,
): Uint8Array {
  const descBytes = new TextEncoder().encode(params.description);

  return ProposalCodec.encode({
    duration: params.duration,
    voteCellCodeHash: config.voteTypeScript.codeHash,
    voteCellHashType: hashTypeToByte(config.voteTypeScript.hashType),
    description: ccc.hexFrom(descBytes),
    receiver: receiverLock,
    amount,
    minimalRequirement,
  });
}

// ─── Consume Proposal ────────────────────────────────────────────────────────

export interface ConsumeProposalParams {
  proposalOutPoint: { txHash: string; index: number };
  /** Hex bytes of the SP1 PLONK proof (contents of proof-plonk.bin) */
  proof: string;
  /** Hex bytes of the molecule-encoded PublicValues (contents of public-values.bin) */
  publicValues: string;
  startBlockHash: string;
  endBlockHash: string;
  config?: Partial<NetworkConfig>;
}

export interface ConsumeProposalResult {
  txHash: string;
}

/**
 * Consume a proposal cell by submitting an SP1 PLONK proof.
 *
 * The proof and publicValues are raw bytes produced by the SP1 prover.
 * The PublicValues bytes are decoded then re-encoded inside ProposalWitness.
 *
 * The witness is placed in WitnessArgs.input_type at the position of the
 * proposal cell input.
 */
export async function consumeProposal(
  signer: ccc.Signer,
  params: ConsumeProposalParams,
): Promise<ConsumeProposalResult> {
  const config = mergeConfig(DEVNET_CONFIG, params.config);
  const client = signer.client;

  // Fetch the proposal cell to read its data
  const proposalCell = await getRequiredCell(client, params.proposalOutPoint);
  if (!proposalCell.outputData || proposalCell.outputData === "0x") {
    throw new Error("Proposal cell has no data");
  }

  const proposalData = ProposalCodec.decode(proposalCell.outputData);
  const receiverLock = proposalData.receiver;

  // Decode the SP1 public_values bytes into a structured PublicValues object
  const publicValuesDecoded = PublicValuesCodec.decode(params.publicValues);

  // Encode the full ProposalWitness
  const witnessBytes = ProposalWitnessCodec.encode({
    proof: params.proof,
    publicValues: publicValuesDecoded,
  });
  const witnessHex = ccc.hexFrom(witnessBytes);

  // Build transaction
  const tx = ccc.Transaction.from({
    inputs: [
      {
        previousOutput: {
          txHash: params.proposalOutPoint.txHash,
          index: params.proposalOutPoint.index,
        },
      },
    ],
    headerDeps: [params.startBlockHash, params.endBlockHash],
    outputs: [
      {
        capacity: proposalData.amount,
        lock: receiverLock,
      },
    ],
    outputsData: ["0x"],
  });

  // Witness for proposal cell input (index 0): put ProposalWitness in input_type
  tx.setWitnessArgsAt(0, ccc.WitnessArgs.from({ inputType: witnessHex }));

  // Cell deps: proposal-type-script contract + always-success lock contract
  tx.addCellDeps(cellDepFromInfo(config.proposalTypeScript));
  tx.addCellDeps(cellDepFromInfo(config.alwaysSuccess));

  // Collect additional inputs for fees (the proposal cell output goes to receiver,
  // so we might need the sender's cells for tx fee)
  await tx.completeFeeBy(signer, config.feeRate);

  const txHash = await signer.sendTransaction(tx);
  return { txHash };
}

// ─── Query helpers ────────────────────────────────────────────────────────────

/**
 * Find all live proposal cells on chain by scanning for cells with the
 * proposal type script code_hash.
 */
export async function* findProposalCells(
  client: ccc.Client,
  config: NetworkConfig = DEVNET_CONFIG,
): AsyncGenerator<ccc.Cell> {
  const typeScript = scriptFromInfo(config.proposalTypeScript);
  // Search by type script prefix (code_hash + hash_type only, any args)
  for await (const cell of client.findCellsByType(
    ccc.Script.from({
      codeHash: typeScript.codeHash,
      hashType: typeScript.hashType,
      args: "0x",
    }),
    true,
  )) {
    yield cell;
  }
}

/**
 * Build a NetworkConfig that overrides only the RPC URL, keeping all other
 * devnet defaults. Useful when talking to testnet/mainnet with the same scripts.
 */
export function configWithRpcUrl(
  rpcUrl: string,
  base: NetworkConfig = DEVNET_CONFIG,
): NetworkConfig {
  return { ...base, ckbRpcUrl: rpcUrl };
}

export { buildClient };
