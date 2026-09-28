// Task 1.3: the "total_staked" counter kept as an Attribute on the Collection

mod common;
use common::{core::*, staking::*, *};

use {
    mpl_core::types::{Plugin, PluginAuthority, PluginType},
    solana_signer::Signer,
};

#[test]
fn test_config_puts_the_counter_on_the_collection() {
    let mut env = setup();
    let s = setup_staking(&mut env);

    // the collection now carries an Attributes plugin starting at zero
    assert_eq!(total_staked(&env, &s.collection), Some(0));

    // and the config pda owns that plugin, so only the program can move it
    let (authority, plugin) =
        collection_plugin(&env, &s.collection, PluginType::Attributes).unwrap();
    assert_eq!(
        authority,
        PluginAuthority::Address {
            address: to_core_key(s.config)
        }
    );
    match plugin {
        Plugin::Attributes(attrs) => {
            assert_eq!(attrs.attribute_list.len(), 1);
            assert_eq!(attrs.attribute_list[0].key, "total_staked");
            assert_eq!(attrs.attribute_list[0].value, "0");
        }
        _ => panic!("expected an Attributes plugin"),
    }
}

#[test]
fn test_counter_goes_up_on_stake_and_down_on_unstake() {
    let mut env = setup();
    let s = setup_staking(&mut env);
    let user = s.user.insecure_clone();

    send(&mut env, &[stake_ix(&s)], &[&user]).unwrap();
    assert_eq!(total_staked(&env, &s.collection), Some(1));

    send(&mut env, &[unstake_ix(&s)], &[&user]).unwrap();
    assert_eq!(total_staked(&env, &s.collection), Some(0));
}

#[test]
fn test_counter_tracks_several_nfts() {
    let mut env = setup();
    let first = setup_staking(&mut env);
    let user = first.user.insecure_clone();
    let second = extra_asset(&mut env, &first);
    let third = extra_asset(&mut env, &first);

    send(&mut env, &[stake_ix(&first)], &[&user]).unwrap();
    send(&mut env, &[stake_ix(&second)], &[&user]).unwrap();
    send(&mut env, &[stake_ix(&third)], &[&user]).unwrap();
    assert_eq!(total_staked(&env, &first.collection), Some(3));

    // unstake one, burn another
    send(&mut env, &[unstake_ix(&second)], &[&user]).unwrap();
    assert_eq!(total_staked(&env, &first.collection), Some(2));

    send(&mut env, &[burn_ix(&third)], &[&user]).unwrap();
    assert_eq!(total_staked(&env, &first.collection), Some(1));

    // the first one is still staked
    assert!(is_frozen(&env, &first.asset));
}

#[test]
fn test_burning_a_staked_nft_lowers_the_counter() {
    let mut env = setup();
    let s = setup_staking(&mut env);
    let user = s.user.insecure_clone();

    send(&mut env, &[stake_ix(&s)], &[&user]).unwrap();
    assert_eq!(total_staked(&env, &s.collection), Some(1));

    send(&mut env, &[burn_ix(&s)], &[&user]).unwrap();
    assert_eq!(total_staked(&env, &s.collection), Some(0));
    assert!(is_burned(&env, &s.asset));
}

#[test]
fn test_claiming_does_not_touch_the_counter() {
    let mut env = setup();
    let s = setup_staking(&mut env);
    let user = s.user.insecure_clone();
    send(&mut env, &[stake_ix(&s)], &[&user]).unwrap();
    warp_seconds(&mut env, DAY);

    let claim = program_ix(
        nft_staking::accounts::ClaimRewards {
            owner: to_core_key(user.pubkey()),
            asset: to_core_key(s.asset),
            config: to_core_key(s.config),
            stake_account: to_core_key(stake_pda(&s.asset)),
            reward_mint: to_core_key(s.reward_mint),
            owner_rewards: to_core_key(reward_ata(&user.pubkey(), &s.reward_mint)),
            token_program: anchor_spl::token::ID,
            associated_token_program: anchor_spl::associated_token::ID,
            system_program: anchor_lang::system_program::ID,
        },
        nft_staking::instruction::ClaimRewards {},
    );
    send(&mut env, &[claim], &[&user]).unwrap();

    // still staked, so still 1
    assert_eq!(total_staked(&env, &s.collection), Some(1));
}

#[test]
fn test_admin_cannot_change_the_counter_by_hand() {
    let mut env = setup();
    let s = setup_staking(&mut env);
    let user = s.user.insecure_clone();
    send(&mut env, &[stake_ix(&s)], &[&user]).unwrap();

    // the admin is still the collection update authority, but the Attributes
    // plugin belongs to the config pda now, so this must fail
    let admin = env.admin.insecure_clone();
    let ix = mpl_core::instructions::UpdateCollectionPluginV1Builder::new()
        .collection(to_core_key(s.collection))
        .payer(to_core_key(env.payer.pubkey()))
        .authority(Some(to_core_key(admin.pubkey())))
        .plugin(Plugin::Attributes(mpl_core::types::Attributes {
            attribute_list: vec![mpl_core::types::Attribute {
                key: "total_staked".to_string(),
                value: "9999".to_string(),
            }],
        }))
        .instruction();

    let res = send(&mut env, &[to_svm_ix(ix)], &[&admin]);
    assert!(res.is_err(), "only the config pda may move the counter");
    assert_eq!(total_staked(&env, &s.collection), Some(1));
}
