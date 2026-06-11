# E2E Tests

The `.ckb-cli` folder contains a pre-existing account whose private key is stored in `pk1`. Its cells are pre-funded by the devnet. All operations below are based on this account.

## Devnet

Steps to run e2e tests on devnet:

1. Run `start.sh` to start the CKB node and miner.
2. Use `deploy.sh` in the `deployment/devnet` folder to deploy or upgrade scripts. For a first-time deployment, remove the `migrations` folder; keep it for upgrades. After the first deployment, update `config.ts` in the SDK, then run `pnpm dev check` to verify the config.
3. Run `dao-deposit.sh` to deposit some into the DAO. Vote weight is based on the deposit amount.
4. In `sdk`, run `pnpm dev create-proposal` to create a proposal.
5. In `sdk`, run `pnpm dev vote` to cast a vote on the proposal.
6. In `sdk`, run `pnpm dev dump-blocks` to fetch blocks and save them as `blocks.bin`.
7. In `sp1/ckb-vote-verification/script`, run `cargo run --release -- --execute --proposal-tx-hash <...> --proposal-index <...> --input blocks.bin`
8. Optional: run `continue-devnet-proof.sh` to generate an SP1 proof via the prover network. This costs real money and requires the `NETWORK_PRIVATE_KEY` environment variable to be set. Most users can stop at step 7.


