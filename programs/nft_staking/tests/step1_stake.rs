// The base staking program: set up a config, lock an nft, unlock it again.

use anchor_lang::AccountDeserialize;
mod common;
use common::{core::*, staking::*, *};

use {
    mpl_core::types::{PluginAuthority, PluginType},
    solana_keypair::Keypair,
    solana_signer::Signer,
};

#[test]
fn test_initialize_config() {
    let mut env = setup();
    let collection = create_collection(&mut env, vec![]).pubkey();
    let config = config_pda(&collection);
    let reward_mint = reward_mint_pda(&config);

    let ix = program_ix(
        nft_staking::accounts::InitializeConfig {
            authority: to_core_key(env.admin.pubkey()),
            collection: to_core_key(collection),
            config: to_core_key(config),
            reward_mint: to_core_key(reward_mint),
            mpl_core_program: mpl_core::ID,
            token_program: anchor_spl::token::ID,
            system_program: anchor_lang::system_program::ID,
        },
        nft_staking::instruction::InitializeConfig {
            points_per_day: POINTS_PER_DAY,
            burn_bonus: BURN_BONUS,
        },
    );
    let admin = env.admin.insecure_clone();
    send(&mut env, &[ix], &[&admin]).unwrap();

    let data = account_data(&env, &config);
    let cfg = nft_staking::StakeConfig::try_deserialize(&mut data.as_slice()).unwrap();
    assert_eq!(cfg.authority, to_core_key(env.admin.pubkey()));
    assert_eq!(cfg.collection, to_core_key(collection));
    assert_eq!(cfg.reward_mint, to_core_key(reward_mint));
    assert_eq!(cfg.points_per_day, POINTS_PER_DAY);
    assert_eq!(cfg.burn_bonus, BURN_BONUS);
    assert!(exists(&env, &reward_mint));
}

#[test]
fn test_stake_freezes_the_nft() {
    let mut env = setup();
    let s = setup_staking(&mut env);

    // before staking the nft is free
    assert!(!is_frozen(&env, &s.asset));
    assert!(asset_plugin(&env, &s.asset, PluginType::FreezeDelegate).is_none());

    let user = s.user.insecure_clone();
    send(&mut env, &[stake_ix(&s)], &[&user]).unwrap();

    // now it is frozen, and the stake pda is the freeze delegate
    assert!(is_frozen(&env, &s.asset));
    let (authority, _) = asset_plugin(&env, &s.asset, PluginType::FreezeDelegate).unwrap();
    assert_eq!(
        authority,
        PluginAuthority::Address {
            address: to_core_key(stake_pda(&s.asset))
        }
    );

    // the user still owns it, it is just locked
    assert_eq!(asset_owner(&env, &s.asset), s.user.pubkey());

    // and the stake account remembers who staked it
    let data = account_data(&env, &stake_pda(&s.asset));
    let stake = nft_staking::StakeAccount::try_deserialize(&mut data.as_slice()).unwrap();
    assert_eq!(stake.owner, to_core_key(s.user.pubkey()));
    assert_eq!(stake.asset, to_core_key(s.asset));
    assert_eq!(stake.staked_at, now(&env));
    assert_eq!(stake.last_claim, now(&env));
}

#[test]
fn test_frozen_nft_cannot_be_transferred() {
    let mut env = setup();
    let s = setup_staking(&mut env);
    let user = s.user.insecure_clone();
    send(&mut env, &[stake_ix(&s)], &[&user]).unwrap();

    // a normal core transfer must fail while the nft is frozen
    let buyer = Keypair::new();
    let ix = mpl_core::instructions::TransferV1Builder::new()
        .asset(to_core_key(s.asset))
        .collection(Some(to_core_key(s.collection)))
        .payer(to_core_key(env.payer.pubkey()))
        .authority(Some(to_core_key(user.pubkey())))
        .new_owner(to_core_key(buyer.pubkey()))
        .instruction();

    let res = send(&mut env, &[to_svm_ix(ix)], &[&user]);
    assert!(res.is_err(), "a frozen nft should not be transferable");
    assert_eq!(asset_owner(&env, &s.asset), user.pubkey());
}

#[test]
fn test_unstake_unfreezes_and_pays_rewards() {
    let mut env = setup();
    let s = setup_staking(&mut env);
    let user = s.user.insecure_clone();
    send(&mut env, &[stake_ix(&s)], &[&user]).unwrap();

    // leave it staked for 3 days
    warp_seconds(&mut env, 3 * DAY);

    send(&mut env, &[unstake_ix(&s)], &[&user]).unwrap();

    // the freeze plugin is gone and the nft is free again
    assert!(!is_frozen(&env, &s.asset));
    assert!(asset_plugin(&env, &s.asset, PluginType::FreezeDelegate).is_none());
    assert_eq!(asset_owner(&env, &s.asset), user.pubkey());

    // 3 days at 10 tokens a day
    let ata = reward_ata(&user.pubkey(), &s.reward_mint);
    assert_eq!(token_balance(&env, &ata), 3 * POINTS_PER_DAY);

    // the stake account is closed and the rent went back to the user
    assert!(!exists(&env, &stake_pda(&s.asset)));
}

#[test]
fn test_only_the_staker_can_unstake() {
    let mut env = setup();
    let s = setup_staking(&mut env);
    let user = s.user.insecure_clone();
    send(&mut env, &[stake_ix(&s)], &[&user]).unwrap();

    // someone else tries to unstake it
    let thief = Keypair::new();
    env.svm.airdrop(&thief.pubkey(), 10_000_000_000).unwrap();
    let mut ix = unstake_ix(&s);
    ix.accounts[0].pubkey = thief.pubkey();
    ix.accounts[6].pubkey = reward_ata(&thief.pubkey(), &s.reward_mint);

    let res = send(&mut env, &[ix], &[&thief]);
    assert!(res.is_err(), "only the staker can unstake");
    assert!(is_frozen(&env, &s.asset));
}

#[test]
fn test_cannot_stake_the_same_nft_twice() {
    let mut env = setup();
    let s = setup_staking(&mut env);
    let user = s.user.insecure_clone();
    send(&mut env, &[stake_ix(&s)], &[&user]).unwrap();

    // the stake pda already exists, so this fails
    let res = send(&mut env, &[stake_ix(&s)], &[&user]);
    assert!(res.is_err(), "an nft cannot be staked twice");
}
