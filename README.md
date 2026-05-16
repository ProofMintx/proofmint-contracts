# StellarCrop Contracts

Soroban smart contracts for StellarCrop’s tokenized warehouse receipt lifecycle.

## Contract Scope

Current contract set includes:
- `receipt-registry`

Target lifecycle:
- Issue receipt
- Transfer receipt ownership
- Redeem (burn/close) receipt
- Manage certified issuer permissions

## Architecture Notes

See:
- `architecture.md`
- `deployment.md`

## Development

```bash
cargo test
```

If you are using the Stellar toolchain locally, ensure `soroban-cli` and Rust target dependencies are installed.

## Current Status

Implemented:
- Workspace scaffold
- Core contract crate and baseline tests

Pending:
- Authorization hardening
- Full event schema for indexer compatibility
- Contract invariants and fuzz/property testing
- Upgrade/governance strategy definition

## Contribution Tracks

- Add tests for edge-case lifecycle transitions
- Define and emit canonical event payloads
- Implement stricter issuer certification controls
- Add admin/ops guardrails and error taxonomy
- Add deployment scripts for testnet workflows

## Related Repositories

- `stellarcrop-indexer`
- `stellarcrop-api`
- `stellarcrop-web`
- `stellarcrop-shared`
