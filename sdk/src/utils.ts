import { ccc } from "@ckb-ccc/shell";
import type { ScriptInfo, NetworkConfig, KnownScriptInfo } from "./config.js";

/**
 * Compute blake160: CKB blake2b-256 hash truncated to 20 bytes.
 * Matches ckb_hash::new_blake2b() (32-byte output) taking the first 20 bytes.
 */
export function blake160(data: ccc.BytesLike): ccc.Hex {
  const hasher = new ccc.HasherCkb();
  hasher.update(data);
  return ccc.hexFrom(ccc.bytesFrom(hasher.digest()).slice(0, 20));
}

/**
 * Compute the CKB type_id (32 bytes) from the first input and output index.
 * Matches ckb-std calculate_type_id: blake2b(CellInput.as_bytes || output_index_u64_le).
 */
export function computeTypeId(
  firstInput: ccc.CellInput,
  outputIndex: number,
): ccc.Hex {
  const indexBuf = new Uint8Array(8);
  new DataView(indexBuf.buffer).setBigUint64(0, BigInt(outputIndex), true);
  return ccc.hashCkb(ccc.CellInput.encode(firstInput), indexBuf);
}

/**
 * Compute the proposal type script args:
 *   typeId[0..20] [20 bytes] || sp1VerifyingKeyHash [32 bytes]
 */
export function buildProposalTypeScriptArgs(
  firstInput: ccc.CellInput,
  outputIndex: number,
  sp1VerifyingKeyHash: string,
): ccc.Hex {
  const typeId = computeTypeId(firstInput, outputIndex);
  // Leading 20 bytes of the 32-byte Type ID (40 hex chars after "0x")
  const typeIdPrefix = typeId.slice(0, 42);
  return ("0x" +
    typeIdPrefix.slice(2) +
    sp1VerifyingKeyHash.slice(2)) as ccc.Hex;
}

/**
 * Build a Script from a ScriptInfo (used for lock/type fields on cells).
 */
export function scriptFromInfo(
  info: ScriptInfo,
  args: ccc.HexLike = "0x",
): ccc.Script {
  return ccc.Script.from({
    codeHash: info.codeHash,
    hashType: info.hashType,
    args: ccc.hexFrom(args),
  });
}

/**
 * Build a CellDep that loads a script contract cell.
 */
export function cellDepFromInfo(info: ScriptInfo): ccc.CellDep {
  return ccc.CellDep.from({
    outPoint: ccc.OutPoint.from({
      txHash: info.outPoint.txHash,
      index: info.outPoint.index,
    }),
    depType: "code",
  });
}

/**
 * Build a CellDep from an OutPointLike (for non-script cells like proposal/DAO cells).
 */
export function cellDepFromOutPoint(op: {
  txHash: string;
  index: number | bigint;
}): ccc.CellDep {
  return ccc.CellDep.from({
    outPoint: ccc.OutPoint.from({ txHash: op.txHash, index: op.index }),
    depType: "code",
  });
}

/**
 * Merge caller-supplied config overrides with the base config.
 */
export function mergeConfig(
  base: NetworkConfig,
  overrides?: Partial<NetworkConfig>,
): NetworkConfig {
  if (!overrides) return base;
  return { ...base, ...overrides };
}

/**
 * Build a CCC client connected to the given RPC URL.
 * Uses ClientPublicTestnet (same `ckt` address prefix as devnet).
 * When the config supplies `knownScripts`, those entries are merged on top of
 * the testnet defaults so only the overridden scripts change; all other
 * known scripts (AnyoneCanPay, xUDT, etc.) remain available.
 */
export function buildClient(
  rpcUrl: string,
  knownScripts?: Record<string, KnownScriptInfo>,
): ccc.ClientPublicTestnet {
  if (!knownScripts) {
    return new ccc.ClientPublicTestnet({ url: rpcUrl });
  }
  // Build a temporary client to read the full testnet script map, then merge.
  const base = new ccc.ClientPublicTestnet();
  const merged = { ...base.scripts, ...knownScripts };
  return new ccc.ClientPublicTestnet({ url: rpcUrl, scripts: merged as never });
}

/**
 * Convert a hash_type string to the byte value used in molecule encoding.
 * CKB spec: data=0, type=1, data1=2, data2=3
 */
export function hashTypeToByte(hashType: string): number {
  switch (hashType) {
    case "data":
      return 0;
    case "type":
      return 1;
    case "data1":
      return 2;
    case "data2":
      return 3;
    default:
      throw new Error(`Unknown hash type: ${hashType}`);
  }
}

/**
 * Fetch a live cell by outpoint. Throws if the cell is not found.
 */
export async function getRequiredCell(
  client: ccc.Client,
  outPoint: { txHash: string; index: number },
): Promise<ccc.Cell> {
  const cell = await client.getCell(
    ccc.OutPoint.from({ txHash: outPoint.txHash, index: outPoint.index }),
  );
  if (!cell) {
    throw new Error(`Cell not found: ${outPoint.txHash}:${outPoint.index}`);
  }
  return cell;
}

/**
 * Get the signer's recommended lock script (secp256k1 by default).
 */
export async function getSignerLock(signer: ccc.Signer): Promise<ccc.Script> {
  const addr = await signer.getRecommendedAddressObj();
  return addr.script;
}
