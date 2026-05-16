# Deployment Guide (Testnet)

## Prerequisites

- Rust and Cargo
- `rustup target add wasm32v1-none`
- Stellar CLI:

```bash
cargo install --locked stellar-cli
```

## Build and test

```bash
npm install
npm run typecheck
npm run test:contracts
npm run build:contracts
```

## Deploy contract

```bash
stellar network add testnet --rpc-url https://soroban-testnet.stellar.org --network-passphrase "Test SDF Network ; September 2015"
stellar keys generate admin
stellar contract deploy \
  --network testnet \
  --source admin \
  --wasm target/wasm32v1-none/release/receipt_registry.wasm
```

Save returned contract ID, then set in `apps/web/.env`:

```bash
NEXT_PUBLIC_RECEIPT_CONTRACT_ID=<contract-id>
NEXT_PUBLIC_STELLAR_NETWORK=testnet
```

## Initialize contract

```bash
stellar contract invoke \
  --network testnet \
  --source admin \
  --id <contract-id> \
  -- init \
  --admin <admin-address>
```

Then add certified issuers with `add_issuer`.
