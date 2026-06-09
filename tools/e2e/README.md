# E2E Tests on Devnet and Testnet


## Devnet

Steps to run e2e tests on devnet:

1. Run `start.sh` to start the CKB node and miner.
2. Use `deploy.sh` in the `deployment/devnet` folder to deploy or upgrade scripts. For a first-time deployment, remove the `migrations` folder; keep it when upgrading.
3. Run `dao-deposit.sh` to deposit into the DAO. Vote weight is based on the deposit amount.
4. In `sdk`, run `pnpm dev create-proposal` to create a proposal.
5. In `sdk`, run `pnpm dev vote` to cast a vote on the proposal.
6. In `sdk`, run `pnpm dev dump-blocks` to fetch blocks and save them as `blocks.bin`.
7. In `sp1/ckb-vote-verification/script`, run `cargo run --release -- --execute --proposal-tx-hash <...> --proposal-index <...> --input blocks.bin`
