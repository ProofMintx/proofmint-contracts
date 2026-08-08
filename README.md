# ProofMint Contracts

The on-chain source of truth for ProofMint credentials.

This repository contains the Rust/Soroban credential registry used by the rest of the ProofMint platform. It is intentionally small: the contract records issuer authorization, recipient, timestamps, revocation, and an immutable 32-byte metadata hash. Credential documents and presentation UI belong in the API and web repositories, not in contract storage.

## Role in ProofMint

The contract is the only authoritative source for credential validity. The indexer listens to its events, the API exposes derived read models, the SDK provides typed access, and the web app supplies issuer/verifier workflows.

Current implementation targets Soroban SDK `26.0.0` and uses Stellar's legacy event publishing API for compatibility with that SDK version.

## Contract Methods

### Constructor
- `__constructor(admin: Address)` — Sets the admin address. The intended production admin is a Stellar multisig account; the constructor runs once at deployment.

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

The indexer consumes the current compact event topics:

| Topic | Payload | Meaning |
|---|---|---|
| `iss_add` | issuer address | Issuer approved |
| `iss_rm` | issuer address | Issuer removed |
| `cred_iss` | credential ID topic; issuer, recipient, hash payload | Credential issued |
| `cred_rev` | credential ID topic; revoking address payload | Credential revoked |

Keep these topics and payload ordering stable when changing the contract. Any ABI change must be reflected in the indexer, SDK, and deployment documentation.

## Storage

- `Admin` — instance storage
- `NextCredentialId` — instance storage
- `Issuer(Address)` — per-issuer persistent storage
- `Credential(u64)` — per-credential persistent storage

Instance and credential entries are extended on writes. The indexer and deployment process must include a durable TTL renewal strategy before production use.

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

The command is a deployment outline. Replace `G...` with the admin address and record the resulting contract ID in the indexer, API, SDK, and web environment configuration. The admin address must be a Stellar multisig account for production use.

## Security Invariants

- Only admin can approve/remove issuers
- Only registered issuers can issue credentials
- Only issuing issuer or admin can revoke
- Metadata hash is immutable after issuance
- Revoked credentials can never return to active

## Related Repositories

- `proofmint-indexer` consumes the events defined here and materializes read models.
- `proofmint-api` serves those read models and off-chain metadata.
- `proofmint-sdk` provides typed contract calls and metadata helpers.
- `proofmint-web` is the issuer and public verification interface.

## License

Apache-2.0
