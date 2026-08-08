#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, BytesN, Env,
};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Admin,
    NextCredentialId,
    Issuer(Address),
    Credential(u64),
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CredentialStatus {
    Active,
    Expired,
    Revoked,
    NotFound,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Credential {
    pub id: u64,
    pub issuer: Address,
    pub recipient: Address,
    pub metadata_hash: BytesN<32>,
    pub issued_at: u64,
    pub expires_at: Option<u64>,
    pub revoked_at: Option<u64>,
}

#[contracterror]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContractError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    IssuerAlreadyRegistered = 4,
    IssuerNotFound = 5,
    CredentialNotFound = 6,
    NotAnIssuer = 7,
    NotIssuerOrAdmin = 8,
    AlreadyRevoked = 9,
    InvalidExpiry = 10,
}

#[contract]
pub struct CredentialRegistry;

#[contractimpl]
impl CredentialRegistry {
    pub fn __constructor(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::NextCredentialId, &1u64);
        env.storage()
            .instance()
            .extend_ttl(120 * 17280, 180 * 17280);
    }

    pub fn register_issuer(env: Env, issuer: Address) -> Result<(), ContractError> {
        let admin = Self::require_admin(&env)?;
        admin.require_auth();

        let key = DataKey::Issuer(issuer.clone());
        if env.storage().persistent().has(&key) {
            return Err(ContractError::IssuerAlreadyRegistered);
        }

        env.storage().persistent().set(&key, &true);
        env.storage()
            .persistent()
            .extend_ttl(&key, 120 * 17280, 180 * 17280);

        env.events()
            .publish((symbol_short!("iss_add"),), issuer);
        Ok(())
    }

    pub fn remove_issuer(env: Env, issuer: Address) -> Result<(), ContractError> {
        let admin = Self::require_admin(&env)?;
        admin.require_auth();

        let key = DataKey::Issuer(issuer.clone());
        if !env.storage().persistent().has(&key) {
            return Err(ContractError::IssuerNotFound);
        }

        env.storage().persistent().remove(&key);

        env.events()
            .publish((symbol_short!("iss_rm"),), issuer);
        Ok(())
    }

    pub fn issue_credential(
        env: Env,
        issuer: Address,
        recipient: Address,
        metadata_hash: BytesN<32>,
        expires_at: Option<u64>,
    ) -> Result<u64, ContractError> {
        issuer.require_auth();
        Self::require_certified_issuer(&env, &issuer)?;

        if let Some(exp) = expires_at {
            if exp <= env.ledger().timestamp() {
                return Err(ContractError::InvalidExpiry);
            }
        }

        let id = Self::next_credential_id(&env)?;
        let now = env.ledger().timestamp();

        let credential = Credential {
            id,
            issuer: issuer.clone(),
            recipient: recipient.clone(),
            metadata_hash: metadata_hash.clone(),
            issued_at: now,
            expires_at,
            revoked_at: None,
        };

        env.storage()
            .persistent()
            .set(&DataKey::Credential(id), &credential);
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::Credential(id), 120 * 17280, 180 * 17280);
        env.storage()
            .instance()
            .set(&DataKey::NextCredentialId, &(id + 1));
        env.storage()
            .instance()
            .extend_ttl(120 * 17280, 180 * 17280);

        env.events().publish(
            (symbol_short!("cred_iss"), id),
            (issuer, recipient, metadata_hash),
        );
        Ok(id)
    }

    pub fn revoke_credential(
        env: Env,
        issuer: Address,
        credential_id: u64,
    ) -> Result<(), ContractError> {
        issuer.require_auth();

        let mut credential = Self::require_credential(&env, credential_id)?;
        if credential.issuer != issuer {
            return Err(ContractError::Unauthorized);
        }
        if credential.revoked_at.is_some() {
            return Err(ContractError::AlreadyRevoked);
        }

        credential.revoked_at = Some(env.ledger().timestamp());
        env.storage()
            .persistent()
            .set(&DataKey::Credential(credential_id), &credential);
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::Credential(credential_id), 120 * 17280, 180 * 17280);

        env.events()
            .publish((symbol_short!("cred_rev"), credential_id), issuer);
        Ok(())
    }

    pub fn admin_revoke_credential(
        env: Env,
        credential_id: u64,
    ) -> Result<(), ContractError> {
        let admin = Self::require_admin(&env)?;
        admin.require_auth();

        let mut credential = Self::require_credential(&env, credential_id)?;
        if credential.revoked_at.is_some() {
            return Err(ContractError::AlreadyRevoked);
        }

        credential.revoked_at = Some(env.ledger().timestamp());
        env.storage()
            .persistent()
            .set(&DataKey::Credential(credential_id), &credential);
        env.storage()
            .persistent()
            .extend_ttl(&DataKey::Credential(credential_id), 120 * 17280, 180 * 17280);

        env.events()
            .publish((symbol_short!("cred_rev"), credential_id), admin);
        Ok(())
    }

    pub fn verify_credential(env: Env, credential_id: u64) -> CredentialStatus {
        let credential = match env
            .storage()
            .persistent()
            .get::<DataKey, Credential>(&DataKey::Credential(credential_id))
        {
            Some(c) => c,
            None => return CredentialStatus::NotFound,
        };

        if credential.revoked_at.is_some() {
            return CredentialStatus::Revoked;
        }

        if let Some(exp) = credential.expires_at {
            if env.ledger().timestamp() >= exp {
                return CredentialStatus::Expired;
            }
        }

        CredentialStatus::Active
    }

    pub fn get_credential(env: Env, credential_id: u64) -> Option<Credential> {
        env.storage()
            .persistent()
            .get::<DataKey, Credential>(&DataKey::Credential(credential_id))
    }

    pub fn is_issuer(env: Env, issuer: Address) -> bool {
        env.storage()
            .persistent()
            .has(&DataKey::Issuer(issuer))
    }

    pub fn get_admin(env: Env) -> Result<Address, ContractError> {
        Self::require_admin(&env)
    }

    fn require_admin(env: &Env) -> Result<Address, ContractError> {
        env.storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::Admin)
            .ok_or(ContractError::NotInitialized)
    }

    fn next_credential_id(env: &Env) -> Result<u64, ContractError> {
        env.storage()
            .instance()
            .get::<DataKey, u64>(&DataKey::NextCredentialId)
            .ok_or(ContractError::NotInitialized)
    }

    fn require_credential(env: &Env, credential_id: u64) -> Result<Credential, ContractError> {
        env.storage()
            .persistent()
            .get::<DataKey, Credential>(&DataKey::Credential(credential_id))
            .ok_or(ContractError::CredentialNotFound)
    }

    fn require_certified_issuer(env: &Env, issuer: &Address) -> Result<(), ContractError> {
        if !env
            .storage()
            .persistent()
            .has(&DataKey::Issuer(issuer.clone()))
        {
            return Err(ContractError::NotAnIssuer);
        }
        Ok(())
    }
}

#[cfg(test)]
mod test;
