// Task 1.2: burn the staked nft for a one time bonus

mod common;
use common::{core::*, staking::*, *};

use {
    mpl_core::types::{PluginAuthority, PluginType},
    solana_keypair::Keypair,
    solana_signer::Signer,
};

#[test]
fn test_stake_adds_a_burn_delegate() {
    let mut env = setup();
    let s = setup_staking(&mut env);
    let user = s.user.insecure_clone();
    send(&mut env, &[stake_ix(&s)], &[&user]).unwrap();

    // the stake pda can burn the nft on the user's behalf
    let (authority, _) = asset_plugin(&env, &s.asset, PluginType::BurnDelegate).unwrap();
    assert_eq!(
        authority,
        PluginAuthority::Address {
            address: to_core_key(stake_pda(&s.asset))
        }
    );
}

#[test]
fn test_burn_pays_the_bonus_and_destroys_the_nft() {
    let mut env = setup();
    let s = setup_staking(&mut env);
    let user = s.user.insecure_clone();
    send(&mut env, &[stake_ix(&s)], &[&user]).unwrap();

    warp_seconds(&mut env, 2 * DAY);
    send(&mut env, &[burn_ix(&s)], &[&user]).unwrap();

    // 2 days of normal rewards plus the burn bonus
    let ata = reward_ata(&user.pubkey(), &s.reward_mint);
    assert_eq!(token_balance(&env, &ata), 2 * POINTS_PER_DAY + BURN_BONUS);

    // the nft is gone for good, and so is the stake account
    assert!(is_burned(&env, &s.asset));
    assert!(!exists(&env, &stake_pda(&s.asset)));
}

#[test]
fn test_burn_works_even_though_the_nft_is_frozen() {
    let mut env = setup();
    let s = setup_staking(&mut env);
    let user = s.user.insecure_clone();
    send(&mut env, &[stake_ix(&s)], &[&user]).unwrap();

    // the nft is frozen right up to the moment we burn it
    assert!(is_frozen(&env, &s.asset));
    send(&mut env, &[burn_ix(&s)], &[&user]).unwrap();
    assert!(is_burned(&env, &s.asset));
}

#[test]
fn test_burn_after_claiming_still_pays_the_full_bonus() {
    let mut env = setup();
    let s = setup_staking(&mut env);
    let user = s.user.insecure_clone();
    send(&mut env, &[stake_ix(&s)], &[&user]).unwrap();
    let ata = reward_ata(&user.pubkey(), &s.reward_mint);

    // claim one day of rewards first
    warp_seconds(&mut env, DAY);
    let claim = program_ix(
        nft_staking::accounts::ClaimRewards {
            owner: to_core_key(user.pubkey()),
            asset: to_core_key(s.asset),
            config: to_core_key(s.config),
            stake_account: to_core_key(stake_pda(&s.asset)),
            reward_mint: to_core_key(s.reward_mint),
            owner_rewards: to_core_key(ata),
            token_program: anchor_spl::token::ID,
            associated_token_program: anchor_spl::associated_token::ID,
            system_program: anchor_lang::system_program::ID,
        },
        nft_staking::instruction::ClaimRewards {},
    );
    send(&mut env, &[claim], &[&user]).unwrap();

    // then burn a day later
    warp_seconds(&mut env, DAY);
    send(&mut env, &[burn_ix(&s)], &[&user]).unwrap();

    // 2 days total plus the full bonus
    assert_eq!(token_balance(&env, &ata), 2 * POINTS_PER_DAY + BURN_BONUS);
}

#[test]
fn test_only_the_staker_can_burn() {
    let mut env = setup();
    let s = setup_staking(&mut env);
    let user = s.user.insecure_clone();
    send(&mut env, &[stake_ix(&s)], &[&user]).unwrap();

    let thief = Keypair::new();
    env.svm.airdrop(&thief.pubkey(), 10_000_000_000).unwrap();
    let mut ix = burn_ix(&s);
    ix.accounts[0].pubkey = thief.pubkey();
    ix.accounts[6].pubkey = reward_ata(&thief.pubkey(), &s.reward_mint);

    let res = send(&mut env, &[ix], &[&thief]);
    assert!(res.is_err(), "only the staker can burn their nft");
    assert!(!is_burned(&env, &s.asset));
}

#[test]
fn test_unstake_removes_the_burn_delegate_too() {
    let mut env = setup();
    let s = setup_staking(&mut env);
    let user = s.user.insecure_clone();
    send(&mut env, &[stake_ix(&s)], &[&user]).unwrap();
    send(&mut env, &[unstake_ix(&s)], &[&user]).unwrap();

    // once unstaked the program has no power over the nft anymore
    assert!(asset_plugin(&env, &s.asset, PluginType::BurnDelegate).is_none());
    assert!(asset_plugin(&env, &s.asset, PluginType::FreezeDelegate).is_none());
}
