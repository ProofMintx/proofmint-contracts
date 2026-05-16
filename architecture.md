# StellarCrop Architecture

## Core model

StellarCrop tracks warehouse commodity receipts as unique Soroban records. The contract is the source of truth for:

- certified issuers
- receipt ownership
- receipt lifecycle state

Receipt lifecycle:

1. `issue`: a certified issuer mints a new active receipt for a farmer wallet.
2. `transfer`: the current owner assigns the receipt to another wallet.
3. `redeem`: owner and issuer co-sign final redemption to mark the receipt immutable.

## Why Soroban records for v1

The system needs one-to-one receipt semantics and per-receipt metadata (store, grade, location, document hash). That is better represented by unique records than by fungible balances.

## Planned extension points

- collateral module that checks ownership and active state before loan issuance
- off-chain indexer for receipt search/history
- optional classic Stellar asset/SAC integration for standardized commodity units
