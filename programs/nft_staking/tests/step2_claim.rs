// Task 1.1: claiming rewards without unstaking

use anchor_lang::AccountDeserialize;
mod common;
use common::{core::*, staking::*, *};

use {mpl_core::types::PluginType, solana_signer::Signer};

fn claim_ix(s: &Staked) -> SvmIx {
    program_ix(
        nft_staking::accounts::ClaimRewards {
            owner: to_core_key(s.user.pubkey()),
            asset: to_core_key(s.asset),
            config: to_core_key(s.config),
            stake_account: to_core_key(stake_pda(&s.asset)),
            reward_mint: to_core_key(s.reward_mint),
            owner_rewards: to_core_key(reward_ata(&s.user.pubkey(), &s.reward_mint)),
            token_program: anchor_spl::token::ID,
            associated_token_program: anchor_spl::associated_token::ID,
            system_program: anchor_lang::system_program::ID,
        },
        nft_staking::instruction::ClaimRewards {},
    )
}

#[test]
fn test_claim_keeps_the_nft_staked_and_frozen() {
    let mut env = setup();
    let s = setup_staking(&mut env);
    let user = s.user.insecure_clone();
    send(&mut env, &[stake_ix(&s)], &[&user]).unwrap();

    warp_seconds(&mut env, 2 * DAY);
    send(&mut env, &[claim_ix(&s)], &[&user]).unwrap();

    // 2 days of rewards landed in the user's ata
    let ata = reward_ata(&user.pubkey(), &s.reward_mint);
    assert_eq!(token_balance(&env, &ata), 2 * POINTS_PER_DAY);

    // and the nft is still staked and still frozen
    assert!(is_frozen(&env, &s.asset));
    assert!(asset_plugin(&env, &s.asset, PluginType::FreezeDelegate).is_some());
    assert!(exists(&env, &stake_pda(&s.asset)));
    assert_eq!(asset_owner(&env, &s.asset), user.pubkey());
}

#[test]
fn test_claim_twice_does_not_pay_the_same_seconds_again() {
    let mut env = setup();
    let s = setup_staking(&mut env);
    let user = s.user.insecure_clone();
    send(&mut env, &[stake_ix(&s)], &[&user]).unwrap();
    let ata = reward_ata(&user.pubkey(), &s.reward_mint);

    // first day
    warp_seconds(&mut env, DAY);
    send(&mut env, &[claim_ix(&s)], &[&user]).unwrap();
    assert_eq!(token_balance(&env, &ata), POINTS_PER_DAY);

    // claiming again right away has nothing to give
    let res = send(&mut env, &[claim_ix(&s)], &[&user]);
    assert_program_error(res, 6003); // NothingToClaim
    assert_eq!(token_balance(&env, &ata), POINTS_PER_DAY);

    // another day, another payout
    warp_seconds(&mut env, DAY);
    send(&mut env, &[claim_ix(&s)], &[&user]).unwrap();
    assert_eq!(token_balance(&env, &ata), 2 * POINTS_PER_DAY);

    // last_claim moved forward
    let data = account_data(&env, &stake_pda(&s.asset));
    let stake = nft_staking::StakeAccount::try_deserialize(&mut data.as_slice()).unwrap();
    assert_eq!(stake.last_claim, now(&env));
}

#[test]
fn test_unstake_after_claiming_only_pays_the_rest() {
    let mut env = setup();
    let s = setup_staking(&mut env);
    let user = s.user.insecure_clone();
    send(&mut env, &[stake_ix(&s)], &[&user]).unwrap();
    let ata = reward_ata(&user.pubkey(), &s.reward_mint);

    // claim after 1 day, then unstake 2 days later
    warp_seconds(&mut env, DAY);
    send(&mut env, &[claim_ix(&s)], &[&user]).unwrap();
    warp_seconds(&mut env, 2 * DAY);
    send(&mut env, &[unstake_ix(&s)], &[&user]).unwrap();

    // 3 days in total, not 4
    assert_eq!(token_balance(&env, &ata), 3 * POINTS_PER_DAY);
    assert!(!is_frozen(&env, &s.asset));
}

#[test]
fn test_only_the_staker_can_claim() {
    let mut env = setup();
    let s = setup_staking(&mut env);
    let user = s.user.insecure_clone();
    send(&mut env, &[stake_ix(&s)], &[&user]).unwrap();
    warp_seconds(&mut env, DAY);

    let thief = solana_keypair::Keypair::new();
    env.svm.airdrop(&thief.pubkey(), 10_000_000_000).unwrap();
    let mut ix = claim_ix(&s);
    ix.accounts[0].pubkey = thief.pubkey();
    ix.accounts[5].pubkey = reward_ata(&thief.pubkey(), &s.reward_mint);

    let res = send(&mut env, &[ix], &[&thief]);
    assert!(res.is_err(), "only the staker can claim");
    assert_eq!(token_balance(&env, &reward_ata(&thief.pubkey(), &s.reward_mint)), 0);
}
