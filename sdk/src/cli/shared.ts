/**
 * Shared helpers for CLI commands.
 */

import { readFileSync } from "node:fs";
import { ccc } from "@ckb-ccc/shell";
import { buildClient } from "../utils.js";
import { DEVNET_CONFIG, type NetworkConfig } from "../config.js";

export const DEFAULT_RPC_URL = DEVNET_CONFIG.ckbRpcUrl;

/**
 * Read a private key from a file.
 * Strips surrounding whitespace / newlines.
 * The file should contain a hex-encoded 32-byte key (with or without 0x prefix).
 */
export function readPrivateKey(filePath: string): string {
  const raw = readFileSync(filePath, "utf8").trim();
  return raw.startsWith("0x") ? raw : `0x${raw}`;
}

/**
 * Build a CCC signer from a private key file and RPC URL.
 * Passes known-script overrides from the config so devnet genesis outpoints
 * are used instead of the hardcoded testnet values.
 */
export function buildSigner(
  privateKeyFile: string,
  config: NetworkConfig,
): ccc.SignerCkbPrivateKey {
  const privateKey = readPrivateKey(privateKeyFile);
  const client = buildClient(config.ckbRpcUrl, config.knownScripts);
  return new ccc.SignerCkbPrivateKey(client, privateKey);
}

/**
 * Read a binary file and return its contents as a "0x..." hex string.
 */
export function readFileAsHex(filePath: string): string {
  const buf = readFileSync(filePath);
  return "0x" + buf.toString("hex");
}

/**
 * Build a NetworkConfig that uses the given RPC URL and devnet script info.
 */
export function configFromRpcUrl(rpcUrl: string): NetworkConfig {
  return { ...DEVNET_CONFIG, ckbRpcUrl: rpcUrl };
}

/**
 * Print a success message and exit 0.
 */
export function success(message: string): never {
  console.log(message);
  process.exit(0);
}

/**
 * Print an error message and exit 1.
 */
export function die(message: string | unknown): never {
  if (message instanceof Error) {
    console.error(`Error: ${message.message}`);
  } else {
    console.error(`Error: ${String(message)}`);
  }
  process.exit(1);
}
