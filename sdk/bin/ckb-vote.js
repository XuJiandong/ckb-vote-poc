#!/usr/bin/env node
/**
 * Thin wrapper that delegates to the TypeScript CLI via tsx.
 * This allows `ckb-vote` to work without a build step during development.
 */
import { spawnSync } from "child_process";
import { fileURLToPath } from "url";
import { dirname, join } from "path";

const __dirname = dirname(fileURLToPath(import.meta.url));
const tsxBin = join(__dirname, "../node_modules/.bin/tsx");
const cliEntry = join(__dirname, "../src/cli/index.ts");

const result = spawnSync(tsxBin, [cliEntry, ...process.argv.slice(2)], {
  stdio: "inherit",
  env: process.env,
});

process.exit(result.status ?? 1);
