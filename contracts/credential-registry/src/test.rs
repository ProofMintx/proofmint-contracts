#![cfg(test)]

extern crate std;

use super::{
    ContractError, CredentialRegistry, CredentialRegistryClient, CredentialStatus,
};
use soroban_sdk::{testutils::Address as _, testutils::Ledger, Address, BytesN, Env};

fn setup() -> (Env, CredentialRegistryClient<'static>, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(CredentialRegistry, (&admin,));
    let client = CredentialRegistryClient::new(&env, &contract_id);

    let issuer = Address::generate(&env);
    let recipient = Address::generate(&env);
    client.register_issuer(&issuer);

    (env, client, admin, issuer, recipient)
}

fn sample_hash(env: &Env) -> BytesN<32> {
    let mut bytes = [0u8; 32];
    for (i, b) in bytes.iter_mut().enumerate() {
        *b = (i % 256) as u8;
    }
    BytesN::from_array(env, &bytes)
}

#[test]
fn initialize_sets_admin() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(CredentialRegistry, (&admin,));
    let client = CredentialRegistryClient::new(&env, &contract_id);

    assert_eq!(client.get_admin(), admin);
}

#[test]
fn constructor_runs_only_once() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let contract_id = env.register(CredentialRegistry, (&admin,));
    let client = CredentialRegistryClient::new(&env, &contract_id);

    assert_eq!(client.get_admin(), admin);
}

#[test]
fn admin_can_register_issuer() {
    let (_env, client, _admin, issuer, _recipient) = setup();

    assert!(client.is_issuer(&issuer));
}

#[test]
fn admin_can_remove_issuer() {
    let (_env, client, _admin, issuer, _recipient) = setup();

    client.remove_issuer(&issuer);
    assert!(!client.is_issuer(&issuer));
}

#[test]
fn cannot_register_issuer_twice() {
    let (_env, client, _admin, issuer, _recipient) = setup();

    let result = client.try_register_issuer(&issuer);
    assert_eq!(result, Err(Ok(ContractError::IssuerAlreadyRegistered)));
}

#[test]
fn non_registered_issuer_cannot_issue() {
    let (_env, client, _admin, _issuer, recipient) = setup();

    let unregistered = Address::generate(&_env);
    let hash = sample_hash(&_env);

    let result = client.try_issue_credential(&unregistered, &recipient, &hash, &None);
    assert_eq!(result, Err(Ok(ContractError::NotAnIssuer)));
}

#[test]
fn issuer_can_issue_credential() {
    let (_env, client, _admin, issuer, recipient) = setup();
    let hash = sample_hash(&_env);

    let id = client.issue_credential(&issuer, &recipient, &hash, &None);
    assert_eq!(id, 1);

    let credential = client.get_credential(&id).unwrap();
    assert_eq!(credential.issuer, issuer);
    assert_eq!(credential.recipient, recipient);
    assert_eq!(credential.metadata_hash, hash);
    assert!(credential.revoked_at.is_none());
}

#[test]
fn issued_credential_is_active() {
    let (_env, client, _admin, issuer, recipient) = setup();
    let hash = sample_hash(&_env);

    let id = client.issue_credential(&issuer, &recipient, &hash, &None);
    let status = client.verify_credential(&id);
    assert!(matches!(status, CredentialStatus::Active));
}

#[test]
fn verify_returns_not_found() {
    let (_env, client, _admin, _issuer, _recipient) = setup();

    let status = client.verify_credential(&999);
    assert!(matches!(status, CredentialStatus::NotFound));
}

#[test]
fn get_credential_returns_none_for_missing() {
    let (_env, client, _admin, _issuer, _recipient) = setup();

    let result = client.get_credential(&999);
    assert!(result.is_none());
}

#[test]
fn issuer_can_revoke_credential() {
    let (_env, client, _admin, issuer, recipient) = setup();
    let hash = sample_hash(&_env);

    let id = client.issue_credential(&issuer, &recipient, &hash, &None);
    client.revoke_credential(&issuer, &id);

    let status = client.verify_credential(&id);
    assert!(matches!(status, CredentialStatus::Revoked));
}

#[test]
fn admin_can_admin_revoke() {
    let (_env, client, _admin, issuer, recipient) = setup();
    let hash = sample_hash(&_env);

    let id = client.issue_credential(&issuer, &recipient, &hash, &None);
    client.admin_revoke_credential(&id);

    let status = client.verify_credential(&id);
    assert!(matches!(status, CredentialStatus::Revoked));
}

#[test]
fn non_issuer_cannot_revoke() {
    let (_env, client, _admin, issuer, recipient) = setup();
    let hash = sample_hash(&_env);

    let id = client.issue_credential(&issuer, &recipient, &hash, &None);

    let stranger = Address::generate(&_env);
    let result = client.try_revoke_credential(&stranger, &id);
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}

#[test]
fn cannot_revoke_already_revoked() {
    let (_env, client, _admin, issuer, recipient) = setup();
    let hash = sample_hash(&_env);

    let id = client.issue_credential(&issuer, &recipient, &hash, &None);
    client.revoke_credential(&issuer, &id);

    let result = client.try_revoke_credential(&issuer, &id);
    assert_eq!(result, Err(Ok(ContractError::AlreadyRevoked)));
}

#[test]
fn expired_credential_is_not_active() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(CredentialRegistry, (&admin,));
    let client = CredentialRegistryClient::new(&env, &contract_id);

    let issuer = Address::generate(&env);
    let recipient = Address::generate(&env);
    client.register_issuer(&issuer);

    let hash = sample_hash(&env);
    let now = env.ledger().timestamp();
    let id = client.issue_credential(&issuer, &recipient, &hash, &Some(now + 3600));

    assert!(matches!(client.verify_credential(&id), CredentialStatus::Active));

    env.ledger().set_timestamp(now + 7200);
    assert!(matches!(client.verify_credential(&id), CredentialStatus::Expired));
}

#[test]
fn cannot_issue_with_past_expiry() {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set_timestamp(1_000_000);
    let admin = Address::generate(&env);
    let contract_id = env.register(CredentialRegistry, (&admin,));
    let client = CredentialRegistryClient::new(&env, &contract_id);

    let issuer = Address::generate(&env);
    let recipient = Address::generate(&env);
    client.register_issuer(&issuer);

    let hash = sample_hash(&env);

    let result = client.try_issue_credential(&issuer, &recipient, &hash, &Some(999_999));
    assert_eq!(result, Err(Ok(ContractError::InvalidExpiry)));
}

#[test]
fn metadata_hash_is_immutable() {
    let (_env, client, _admin, issuer, recipient) = setup();
    let hash = sample_hash(&_env);

    let id = client.issue_credential(&issuer, &recipient, &hash, &None);
    let credential = client.get_credential(&id).unwrap();
    assert_eq!(credential.metadata_hash, hash);

    client.revoke_credential(&issuer, &id);
    let revoked = client.get_credential(&id).unwrap();
    assert_eq!(revoked.metadata_hash, hash);
}

#[test]
fn credential_ids_are_sequential() {
    let (_env, client, _admin, issuer, recipient) = setup();
    let hash = sample_hash(&_env);
    let recipient2 = Address::generate(&_env);

    let id1 = client.issue_credential(&issuer, &recipient, &hash, &None);
    let id2 = client.issue_credential(&issuer, &recipient2, &hash, &None);

    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
}

#[test]
fn removed_issuer_cannot_issue() {
    let (_env, client, _admin, issuer, recipient) = setup();
    let hash = sample_hash(&_env);

    client.remove_issuer(&issuer);
    let result = client.try_issue_credential(&issuer, &recipient, &hash, &None);
    assert_eq!(result, Err(Ok(ContractError::NotAnIssuer)));
}

#[test]
fn revoked_credential_stays_revoked() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register(CredentialRegistry, (&admin,));
    let client = CredentialRegistryClient::new(&env, &contract_id);

    let issuer = Address::generate(&env);
    let recipient = Address::generate(&env);
    client.register_issuer(&issuer);

    let hash = sample_hash(&env);
    let now = env.ledger().timestamp();

    let id = client.issue_credential(&issuer, &recipient, &hash, &Some(now + 3600));
    client.revoke_credential(&issuer, &id);

    env.ledger().set_timestamp(now + 7200);
    assert!(matches!(client.verify_credential(&id), CredentialStatus::Revoked));
}
