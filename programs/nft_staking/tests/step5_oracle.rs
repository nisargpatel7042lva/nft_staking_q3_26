// Task 2: the time based transfer oracle

use anchor_lang::AccountDeserialize;
mod common;
use common::{core::*, staking::*, *};

use {
    mpl_core::types::{ExternalValidationResult, OracleValidation},
    solana_keypair::Keypair,
    solana_signer::Signer,
};

const VAULT_FUNDING: u64 = 100_000_000; // 0.1 SOL for crank rewards
const CRANK_REWARD: u64 = 1_000_000;

fn transfer_state(env: &Env, collection: &SvmKey) -> ExternalValidationResult {
    let data = account_data(env, &oracle_pda(collection));
    let oracle = nft_staking::OracleState::try_deserialize(&mut data.as_slice()).unwrap();
    match oracle.validation {
        OracleValidation::V1 { transfer, .. } => transfer,
        OracleValidation::Uninitialized => panic!("oracle was never initialized"),
    }
}

// sets up staking plus the oracle, with the clock inside trading hours
fn setup_with_oracle(env: &mut Env) -> Staked {
    let s = setup_staking(env);
    let admin = env.admin.insecure_clone();
    send(env, &[init_oracle_ix(&s, &admin.pubkey(), VAULT_FUNDING)], &[&admin]).unwrap();
    s
}

#[test]
fn test_oracle_starts_closed_and_is_attached_to_the_collection() {
    let mut env = setup();
    let s = setup_with_oracle(&mut env);

    // before any crank runs, transfers are rejected
    assert_eq!(transfer_state(&env, &s.collection), ExternalValidationResult::Rejected);

    // the vault got funded
    assert_eq!(
        env.svm.get_account(&vault_pda(&s.collection)).unwrap().lamports,
        VAULT_FUNDING
    );
}

#[test]
fn test_crank_opens_and_closes_with_the_clock() {
    let mut env = setup();
    let s = setup_with_oracle(&mut env);
    let cranker = Keypair::new();
    env.svm.airdrop(&cranker.pubkey(), 10_000_000_000).unwrap();

    // 10:00 UTC is inside 9-17, so the oracle should approve
    set_hour(&mut env, 10, 0);
    send_with_payer(&mut env, &[update_oracle_ix(&s, &cranker.pubkey())], &cranker, &[]).unwrap();
    assert_eq!(transfer_state(&env, &s.collection), ExternalValidationResult::Approved);

    // 20:00 is outside, so it should reject again
    set_hour(&mut env, 20, 0);
    send_with_payer(&mut env, &[update_oracle_ix(&s, &cranker.pubkey())], &cranker, &[]).unwrap();
    assert_eq!(transfer_state(&env, &s.collection), ExternalValidationResult::Rejected);

    // and 03:00 (before opening) still rejects
    set_hour(&mut env, 3, 0);
    send_with_payer(&mut env, &[update_oracle_ix(&s, &cranker.pubkey())], &cranker, &[]).unwrap();
    assert_eq!(transfer_state(&env, &s.collection), ExternalValidationResult::Rejected);
}

#[test]
fn test_anyone_can_crank() {
    let mut env = setup();
    let s = setup_with_oracle(&mut env);

    // a total stranger, nothing to do with the collection
    let stranger = Keypair::new();
    env.svm.airdrop(&stranger.pubkey(), 10_000_000_000).unwrap();
    set_hour(&mut env, 12, 0);

    send_with_payer(&mut env, &[update_oracle_ix(&s, &stranger.pubkey())], &stranger, &[]).unwrap();
    assert_eq!(transfer_state(&env, &s.collection), ExternalValidationResult::Approved);
}

#[test]
fn test_crank_is_paid_at_the_boundary_but_not_in_the_middle_of_the_day() {
    let mut env = setup();
    let s = setup_with_oracle(&mut env);
    let cranker = Keypair::new();
    env.svm.airdrop(&cranker.pubkey(), 10_000_000_000).unwrap();

    // 09:05, just after opening, and the state flips closed -> open: gets paid
    set_hour(&mut env, 9, 5);
    let before = env.svm.get_account(&cranker.pubkey()).unwrap().lamports;
    send_with_payer(&mut env, &[update_oracle_ix(&s, &cranker.pubkey())], &cranker, &[]).unwrap();
    let after = env.svm.get_account(&cranker.pubkey()).unwrap().lamports;
    // reward minus the transaction fee they paid
    assert!(after > before, "cranking at the boundary should pay");
    assert_eq!(
        env.svm.get_account(&vault_pda(&s.collection)).unwrap().lamports,
        VAULT_FUNDING - CRANK_REWARD
    );

    // 13:00, nothing changes, so no payout
    set_hour(&mut env, 13, 0);
    let before = env.svm.get_account(&cranker.pubkey()).unwrap().lamports;
    send_with_payer(&mut env, &[update_oracle_ix(&s, &cranker.pubkey())], &cranker, &[]).unwrap();
    let after = env.svm.get_account(&cranker.pubkey()).unwrap().lamports;
    assert!(after < before, "a pointless crank should only cost the fee");
    assert_eq!(
        env.svm.get_account(&vault_pda(&s.collection)).unwrap().lamports,
        VAULT_FUNDING - CRANK_REWARD
    );
}

#[test]
fn test_transfer_is_allowed_during_trading_hours() {
    let mut env = setup();
    let s = setup_with_oracle(&mut env);
    let user = s.user.insecure_clone();
    let cranker = Keypair::new();
    env.svm.airdrop(&cranker.pubkey(), 10_000_000_000).unwrap();

    set_hour(&mut env, 10, 0);
    send_with_payer(&mut env, &[update_oracle_ix(&s, &cranker.pubkey())], &cranker, &[]).unwrap();

    let buyer = Keypair::new();
    send(&mut env, &[transfer_nft_ix(&s, &buyer.pubkey())], &[&user]).unwrap();
    assert_eq!(asset_owner(&env, &s.asset), buyer.pubkey());
}

#[test]
fn test_transfer_is_blocked_outside_trading_hours() {
    let mut env = setup();
    let s = setup_with_oracle(&mut env);
    let user = s.user.insecure_clone();
    let cranker = Keypair::new();
    env.svm.airdrop(&cranker.pubkey(), 10_000_000_000).unwrap();

    // market closes
    set_hour(&mut env, 20, 0);
    send_with_payer(&mut env, &[update_oracle_ix(&s, &cranker.pubkey())], &cranker, &[]).unwrap();

    let buyer = Keypair::new();
    let res = send(&mut env, &[transfer_nft_ix(&s, &buyer.pubkey())], &[&user]);
    assert!(res.is_err(), "the oracle should block transfers at night");
    assert_eq!(asset_owner(&env, &s.asset), user.pubkey());

    // when the market opens again the same transfer goes through
    set_hour(&mut env, 10, 0);
    send_with_payer(&mut env, &[update_oracle_ix(&s, &cranker.pubkey())], &cranker, &[]).unwrap();
    send(&mut env, &[transfer_nft_ix(&s, &buyer.pubkey())], &[&user]).unwrap();
    assert_eq!(asset_owner(&env, &s.asset), buyer.pubkey());
}

#[test]
fn test_a_plain_core_transfer_is_blocked_too() {
    let mut env = setup();
    let s = setup_with_oracle(&mut env);
    let user = s.user.insecure_clone();
    let cranker = Keypair::new();
    env.svm.airdrop(&cranker.pubkey(), 10_000_000_000).unwrap();
    set_hour(&mut env, 20, 0);
    send_with_payer(&mut env, &[update_oracle_ix(&s, &cranker.pubkey())], &cranker, &[]).unwrap();

    // going straight to core, not through our program, must fail as well
    let buyer = Keypair::new();
    let ix = mpl_core::instructions::TransferV1Builder::new()
        .asset(to_core_key(s.asset))
        .collection(Some(to_core_key(s.collection)))
        .payer(to_core_key(env.payer.pubkey()))
        .authority(Some(to_core_key(user.pubkey())))
        .new_owner(to_core_key(buyer.pubkey()))
        .add_remaining_account(anchor_lang::solana_program::instruction::AccountMeta::new_readonly(
            to_core_key(oracle_pda(&s.collection)),
            false,
        ))
        .instruction();

    let res = send(&mut env, &[to_svm_ix(ix)], &[&user]);
    assert!(res.is_err(), "the oracle guards the asset, not just our program");
    assert_eq!(asset_owner(&env, &s.asset), user.pubkey());
}
