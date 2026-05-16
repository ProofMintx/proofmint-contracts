#![cfg(test)]

extern crate std;

use super::{ContractError, ReceiptInput, ReceiptRegistry, ReceiptRegistryClient, ReceiptStatus};
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn setup() -> (Env, ReceiptRegistryClient<'static>, Address, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(ReceiptRegistry, ());
    let client = ReceiptRegistryClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let issuer = Address::generate(&env);
    let farmer = Address::generate(&env);
    let buyer = Address::generate(&env);

    client.init(&admin);

    (env, client, admin, issuer, farmer, buyer)
}

fn sample_input(env: &Env, owner: &Address) -> ReceiptInput {
    ReceiptInput {
        owner: owner.clone(),
        commodity: String::from_str(env, "maize"),
        quantity_kg: 50,
        grade: String::from_str(env, "A"),
        warehouse_name: String::from_str(env, "AgriStore Ibadan"),
        location: String::from_str(env, "Ibadan, NG"),
        metadata_hash: String::from_str(env, "QmXyZ123"),
        expires_at: env.ledger().timestamp() + 86_400,
    }
}

#[test]
fn admin_can_manage_issuers() {
    let (_env, client, admin, issuer, _farmer, _buyer) = setup();

    client.add_issuer(&issuer);
    assert!(client.is_certified_issuer(&issuer));

    client.remove_issuer(&issuer);
    assert!(!client.is_certified_issuer(&issuer));

    let duplicate = client.try_add_issuer(&issuer);
    assert!(duplicate.is_ok());

    let _ = admin;
}

#[test]
fn uncertified_issuer_cannot_issue() {
    let (env, client, _admin, issuer, farmer, _buyer) = setup();
    let input = sample_input(&env, &farmer);

    let result = client.try_issue(&issuer, &input);
    assert_eq!(result, Err(Ok(ContractError::IssuerNotCertified)));
}

#[test]
fn issue_transfer_redeem_happy_path() {
    let (env, client, _admin, issuer, farmer, buyer) = setup();
    client.add_issuer(&issuer);

    let input = sample_input(&env, &farmer);
    let receipt_id = client.issue(&issuer, &input);

    let receipt = client.get_receipt(&receipt_id);
    assert_eq!(receipt.owner, farmer);
    assert_eq!(receipt.status, ReceiptStatus::Active);

    client.transfer(&farmer, &receipt_id, &buyer);
    let transferred = client.get_receipt(&receipt_id);
    assert_eq!(transferred.owner, buyer);

    client.redeem(&buyer, &issuer, &receipt_id);
    let redeemed = client.get_receipt(&receipt_id);
    assert_eq!(redeemed.status, ReceiptStatus::Redeemed);
}

#[test]
fn non_owner_cannot_transfer() {
    let (env, client, _admin, issuer, farmer, buyer) = setup();
    client.add_issuer(&issuer);

    let input = sample_input(&env, &farmer);
    let receipt_id = client.issue(&issuer, &input);

    let result = client.try_transfer(&buyer, &receipt_id, &issuer);
    assert_eq!(result, Err(Ok(ContractError::Unauthorized)));
}

#[test]
fn redeemed_receipt_cannot_transfer_again() {
    let (env, client, _admin, issuer, farmer, buyer) = setup();
    client.add_issuer(&issuer);

    let input = sample_input(&env, &farmer);
    let receipt_id = client.issue(&issuer, &input);
    client.redeem(&farmer, &issuer, &receipt_id);

    let transfer_after_redeem = client.try_transfer(&farmer, &receipt_id, &buyer);
    assert_eq!(transfer_after_redeem, Err(Ok(ContractError::InvalidState)));

    let redeem_twice = client.try_redeem(&farmer, &issuer, &receipt_id);
    assert_eq!(redeem_twice, Err(Ok(ContractError::InvalidState)));
}
