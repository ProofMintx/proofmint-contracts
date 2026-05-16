#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, Map, String,
    Vec,
};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    Issuers,
    NextReceiptId,
    Receipt(u64),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReceiptStatus {
    Active,
    Redeemed,
    Cancelled,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Receipt {
    pub receipt_id: u64,
    pub issuer: Address,
    pub owner: Address,
    pub commodity: String,
    pub quantity_kg: i128,
    pub grade: String,
    pub warehouse_name: String,
    pub location: String,
    pub metadata_hash: String,
    pub issued_at: u64,
    pub expires_at: u64,
    pub status: ReceiptStatus,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReceiptInput {
    pub owner: Address,
    pub commodity: String,
    pub quantity_kg: i128,
    pub grade: String,
    pub warehouse_name: String,
    pub location: String,
    pub metadata_hash: String,
    pub expires_at: u64,
}

#[contracterror]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContractError {
    AlreadyInitialized = 1,
    Unauthorized = 2,
    IssuerAlreadyCertified = 3,
    IssuerNotCertified = 4,
    ReceiptNotFound = 5,
    InvalidAmount = 6,
    InvalidOwner = 7,
    InvalidState = 8,
}

#[contract]
pub struct ReceiptRegistry;

#[contractimpl]
impl ReceiptRegistry {
    pub fn init(env: Env, admin: Address) -> Result<(), ContractError> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(ContractError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::NextReceiptId, &1u64);
        let issuers: Map<Address, bool> = Map::new(&env);
        env.storage().instance().set(&DataKey::Issuers, &issuers);
        Ok(())
    }

    pub fn add_issuer(env: Env, issuer: Address) -> Result<(), ContractError> {
        let admin = Self::admin(&env)?;
        admin.require_auth();

        let mut issuers = Self::issuers(&env)?;
        if issuers.get(issuer.clone()).unwrap_or(false) {
            return Err(ContractError::IssuerAlreadyCertified);
        }

        issuers.set(issuer.clone(), true);
        env.storage().instance().set(&DataKey::Issuers, &issuers);
        env.events().publish((symbol_short!("issr_add"),), issuer);
        Ok(())
    }

    pub fn remove_issuer(env: Env, issuer: Address) -> Result<(), ContractError> {
        let admin = Self::admin(&env)?;
        admin.require_auth();

        let mut issuers = Self::issuers(&env)?;
        if !issuers.get(issuer.clone()).unwrap_or(false) {
            return Err(ContractError::IssuerNotCertified);
        }

        issuers.set(issuer.clone(), false);
        env.storage().instance().set(&DataKey::Issuers, &issuers);
        env.events().publish((symbol_short!("issuer_rm"),), issuer);
        Ok(())
    }

    pub fn issue(env: Env, issuer: Address, input: ReceiptInput) -> Result<u64, ContractError> {
        issuer.require_auth();
        Self::require_certified_issuer(&env, &issuer)?;

        if input.quantity_kg <= 0 {
            return Err(ContractError::InvalidAmount);
        }

        if input.owner == issuer {
            return Err(ContractError::InvalidOwner);
        }

        let receipt_id = Self::next_receipt_id(&env)?;
        let now = env.ledger().timestamp();

        let receipt = Receipt {
            receipt_id,
            issuer: issuer.clone(),
            owner: input.owner.clone(),
            commodity: input.commodity,
            quantity_kg: input.quantity_kg,
            grade: input.grade,
            warehouse_name: input.warehouse_name,
            location: input.location,
            metadata_hash: input.metadata_hash,
            issued_at: now,
            expires_at: input.expires_at,
            status: ReceiptStatus::Active,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Receipt(receipt_id), &receipt);
        env.storage()
            .instance()
            .set(&DataKey::NextReceiptId, &(receipt_id + 1));

        env.events().publish(
            (symbol_short!("issued"), receipt_id),
            (issuer, input.owner),
        );

        Ok(receipt_id)
    }

    pub fn transfer(
        env: Env,
        owner: Address,
        receipt_id: u64,
        new_owner: Address,
    ) -> Result<(), ContractError> {
        owner.require_auth();

        let mut receipt = Self::receipt(&env, receipt_id)?;
        if receipt.status != ReceiptStatus::Active {
            return Err(ContractError::InvalidState);
        }
        if receipt.owner != owner {
            return Err(ContractError::Unauthorized);
        }
        if new_owner == owner {
            return Err(ContractError::InvalidOwner);
        }

        receipt.owner = new_owner.clone();
        env.storage()
            .persistent()
            .set(&DataKey::Receipt(receipt_id), &receipt);

        env.events().publish(
            (symbol_short!("transfer"), receipt_id),
            (owner, new_owner),
        );

        Ok(())
    }

    pub fn redeem(
        env: Env,
        owner: Address,
        issuer: Address,
        receipt_id: u64,
    ) -> Result<(), ContractError> {
        owner.require_auth();
        issuer.require_auth();

        let mut receipt = Self::receipt(&env, receipt_id)?;
        if receipt.status != ReceiptStatus::Active {
            return Err(ContractError::InvalidState);
        }
        if receipt.owner != owner {
            return Err(ContractError::Unauthorized);
        }
        if receipt.issuer != issuer {
            return Err(ContractError::Unauthorized);
        }

        receipt.status = ReceiptStatus::Redeemed;
        env.storage()
            .persistent()
            .set(&DataKey::Receipt(receipt_id), &receipt);

        env.events()
            .publish((symbol_short!("redeemed"), receipt_id), (owner, issuer));

        Ok(())
    }

    pub fn get_receipt(env: Env, receipt_id: u64) -> Result<Receipt, ContractError> {
        Self::receipt(&env, receipt_id)
    }

    pub fn get_owner(env: Env, receipt_id: u64) -> Result<Address, ContractError> {
        let receipt = Self::receipt(&env, receipt_id)?;
        Ok(receipt.owner)
    }

    pub fn is_certified_issuer(env: Env, issuer: Address) -> Result<bool, ContractError> {
        let issuers = Self::issuers(&env)?;
        Ok(issuers.get(issuer).unwrap_or(false))
    }

    pub fn list_receipts_for_owner(
        env: Env,
        owner: Address,
        from_id: u64,
        limit: u32,
    ) -> Result<Vec<Receipt>, ContractError> {
        let mut out = Vec::new(&env);
        let next = Self::next_receipt_id(&env)?;
        if next <= from_id || limit == 0 {
            return Ok(out);
        }

        let mut scanned = 0u32;
        let mut current = from_id;
        while current < next && scanned < limit {
            if let Some(receipt) = env
                .storage()
                .persistent()
                .get::<DataKey, Receipt>(&DataKey::Receipt(current))
            {
                if receipt.owner == owner {
                    out.push_back(receipt);
                }
            }
            current += 1;
            scanned += 1;
        }

        Ok(out)
    }

    fn admin(env: &Env) -> Result<Address, ContractError> {
        env.storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::Admin)
            .ok_or(ContractError::Unauthorized)
    }

    fn issuers(env: &Env) -> Result<Map<Address, bool>, ContractError> {
        env.storage()
            .instance()
            .get::<DataKey, Map<Address, bool>>(&DataKey::Issuers)
            .ok_or(ContractError::Unauthorized)
    }

    fn next_receipt_id(env: &Env) -> Result<u64, ContractError> {
        env.storage()
            .instance()
            .get::<DataKey, u64>(&DataKey::NextReceiptId)
            .ok_or(ContractError::Unauthorized)
    }

    fn receipt(env: &Env, receipt_id: u64) -> Result<Receipt, ContractError> {
        env.storage()
            .persistent()
            .get::<DataKey, Receipt>(&DataKey::Receipt(receipt_id))
            .ok_or(ContractError::ReceiptNotFound)
    }

    fn require_certified_issuer(env: &Env, issuer: &Address) -> Result<(), ContractError> {
        let issuers = Self::issuers(env)?;
        if !issuers.get(issuer.clone()).unwrap_or(false) {
            return Err(ContractError::IssuerNotCertified);
        }
        Ok(())
    }
}

#[cfg(test)]
mod test;
