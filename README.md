# ProofMint Contracts

Soroban credential registry for verifiable credentials on Stellar.

## Overview

The credential registry is a Soroban smart contract that manages the full lifecycle of verifiable credentials on Stellar. It stores credential metadata hashes on-chain while keeping sensitive data off-chain.

## Contract Methods

### Constructor
- `__constructor(admin: Address)` — Sets the admin address (multisig wallet). Runs once at deployment.

### Issuer Management
- `register_issuer(issuer: Address)` — Admin approves an issuer
- `remove_issuer(issuer: Address)` — Admin removes an issuer
- `is_issuer(issuer: Address) -> bool` — Check issuer status

### Credential Lifecycle
- `issue_credential(issuer, recipient, metadata_hash, expires_at?) -> u64` — Issues a new credential
- `revoke_credential(issuer, credential_id)` — Issuer revokes their own credential
- `admin_revoke_credential(credential_id)` — Admin revokes any credential

### Verification
- `verify_credential(credential_id) -> CredentialStatus` — Returns Active, Expired, Revoked, or NotFound
- `get_credential(credential_id) -> Option<Credential>` — Full credential details

### Admin
- `get_admin() -> Address` — Returns the admin address

## Credential Status

| Status | Condition |
|--------|-----------|
| Active | Issued, not expired, not revoked |
| Expired | expiry timestamp has passed |
| Revoked | revoked_at is set (can never return to active) |
| NotFound | credential ID does not exist |

## Events

- `IssuerRegistered { issuer }`
- `IssuerRemoved { issuer }`
- `CredentialIssued { credential_id, issuer, recipient, metadata_hash, expires_at? }`
- `CredentialRevoked { credential_id, revoked_by }`

## Storage

- `Admin` — instance storage
- `NextCredentialId` — instance storage
- `Issuer(Address)` — per-issuer persistent storage
- `Credential(u64)` — per-credential persistent storage

TTL is extended to ~30 days (120 ledgers) on write and extended to ~45 days on read.

## Development

```bash
rustup target add wasm32v1-none
cargo test
stellar contract build
```

## Deploy to Testnet

```bash
stellar contract build
stellar keys generate admin --network testnet --fund
stellar contract deploy \
  --wasm target/wasm32v1-none/release/credential_registry.wasm \
  --source-account admin \
  --network testnet \
  -- \
  --admin G...
```

The admin address must be a Stellar multisig account for production use.

## Security Invariants

- Only admin can approve/remove issuers
- Only registered issuers can issue credentials
- Only issuing issuer or admin can revoke
- Metadata hash is immutable after issuance
- Revoked credentials can never return to active

## License

Apache-2.0
