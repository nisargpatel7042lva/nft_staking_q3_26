// Shared helpers for building our program's instructions and setting up a
// collection + config + nft ready to stake.
#![allow(dead_code)]

use {
    super::{core::*, *},
    solana_keypair::Keypair,
    solana_signer::Signer,
};

// derives the pdas our program uses
pub fn config_pda(collection: &SvmKey) -> SvmKey {
    let (key, _) = CoreKey::find_program_address(
        &[b"config", to_core_key(*collection).as_ref()],
        &nft_staking::ID,
    );
    to_svm_key(key)
}

pub fn reward_mint_pda(config: &SvmKey) -> SvmKey {
    let (key, _) = CoreKey::find_program_address(
        &[b"rewards", to_core_key(*config).as_ref()],
        &nft_staking::ID,
    );
    to_svm_key(key)
}

pub fn stake_pda(asset: &SvmKey) -> SvmKey {
    let (key, _) =
        CoreKey::find_program_address(&[b"stake", to_core_key(*asset).as_ref()], &nft_staking::ID);
    to_svm_key(key)
}

// full setup: collection + config + one nft owned by `user`
pub struct Staked {
    pub collection: SvmKey,
    pub config: SvmKey,
    pub reward_mint: SvmKey,
    pub user: Keypair,
    pub asset: SvmKey,
}

pub fn setup_staking(env: &mut Env) -> Staked {
    let collection = create_collection(env, vec![]).pubkey();
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
    send(env, &[ix], &[&admin]).unwrap();

    let user = Keypair::new();
    env.svm.airdrop(&user.pubkey(), 100_000_000_000).unwrap();
    let asset = create_asset(env, &collection, &user.pubkey()).pubkey();

    Staked {
        collection,
        config,
        reward_mint,
        user,
        asset,
    }
}

pub fn stake_ix(s: &Staked) -> SvmIx {
    program_ix(
        nft_staking::accounts::Stake {
            owner: to_core_key(s.user.pubkey()),
            asset: to_core_key(s.asset),
            collection: to_core_key(s.collection),
            config: to_core_key(s.config),
            stake_account: to_core_key(stake_pda(&s.asset)),
            mpl_core_program: mpl_core::ID,
            system_program: anchor_lang::system_program::ID,
        },
        nft_staking::instruction::Stake {},
    )
}

pub fn unstake_ix(s: &Staked) -> SvmIx {
    program_ix(
        nft_staking::accounts::Unstake {
            owner: to_core_key(s.user.pubkey()),
            asset: to_core_key(s.asset),
            collection: to_core_key(s.collection),
            config: to_core_key(s.config),
            stake_account: to_core_key(stake_pda(&s.asset)),
            reward_mint: to_core_key(s.reward_mint),
            owner_rewards: to_core_key(reward_ata(&s.user.pubkey(), &s.reward_mint)),
            mpl_core_program: mpl_core::ID,
            token_program: anchor_spl::token::ID,
            associated_token_program: anchor_spl::associated_token::ID,
            system_program: anchor_lang::system_program::ID,
        },
        nft_staking::instruction::Unstake {},
    )
}


pub fn burn_ix(s: &Staked) -> SvmIx {
    program_ix(
        nft_staking::accounts::BurnStakedNft {
            owner: to_core_key(s.user.pubkey()),
            asset: to_core_key(s.asset),
            collection: to_core_key(s.collection),
            config: to_core_key(s.config),
            stake_account: to_core_key(stake_pda(&s.asset)),
            reward_mint: to_core_key(s.reward_mint),
            owner_rewards: to_core_key(reward_ata(&s.user.pubkey(), &s.reward_mint)),
            mpl_core_program: mpl_core::ID,
            token_program: anchor_spl::token::ID,
            associated_token_program: anchor_spl::associated_token::ID,
            system_program: anchor_lang::system_program::ID,
        },
        nft_staking::instruction::BurnStakedNft {},
    )
}

// a second (third, ...) nft in the same collection, staked by the same user
pub fn extra_asset(env: &mut Env, s: &Staked) -> Staked {
    let asset = create_asset(env, &s.collection, &s.user.pubkey()).pubkey();
    Staked {
        collection: s.collection,
        config: s.config,
        reward_mint: s.reward_mint,
        user: s.user.insecure_clone(),
        asset,
    }
}

pub fn oracle_pda(collection: &SvmKey) -> SvmKey {
    let (key, _) = CoreKey::find_program_address(
        &[b"oracle", to_core_key(*collection).as_ref()],
        &nft_staking::ID,
    );
    to_svm_key(key)
}

pub fn vault_pda(collection: &SvmKey) -> SvmKey {
    let (key, _) = CoreKey::find_program_address(
        &[b"vault", to_core_key(*collection).as_ref()],
        &nft_staking::ID,
    );
    to_svm_key(key)
}

pub fn init_oracle_ix(s: &Staked, admin: &SvmKey, funding: u64) -> SvmIx {
    program_ix(
        nft_staking::accounts::InitializeOracle {
            authority: to_core_key(*admin),
            collection: to_core_key(s.collection),
            oracle: to_core_key(oracle_pda(&s.collection)),
            vault: to_core_key(vault_pda(&s.collection)),
            mpl_core_program: mpl_core::ID,
            system_program: anchor_lang::system_program::ID,
        },
        nft_staking::instruction::InitializeOracle {
            vault_funding: funding,
        },
    )
}

pub fn update_oracle_ix(s: &Staked, cranker: &SvmKey) -> SvmIx {
    program_ix(
        nft_staking::accounts::UpdateOracle {
            cranker: to_core_key(*cranker),
            collection: to_core_key(s.collection),
            oracle: to_core_key(oracle_pda(&s.collection)),
            vault: to_core_key(vault_pda(&s.collection)),
            system_program: anchor_lang::system_program::ID,
        },
        nft_staking::instruction::UpdateOracle {},
    )
}

pub fn transfer_nft_ix(s: &Staked, new_owner: &SvmKey) -> SvmIx {
    program_ix(
        nft_staking::accounts::TransferNft {
            owner: to_core_key(s.user.pubkey()),
            asset: to_core_key(s.asset),
            collection: to_core_key(s.collection),
            new_owner: to_core_key(*new_owner),
            oracle: to_core_key(oracle_pda(&s.collection)),
            mpl_core_program: mpl_core::ID,
            system_program: anchor_lang::system_program::ID,
        },
        nft_staking::instruction::TransferNft {},
    )
}

// moves the clock to a given hour of the day (UTC), keeping the same day
pub fn set_hour(env: &mut Env, hour: i64, minute: i64) {
    let mut clock = env.svm.get_sysvar::<solana_clock::Clock>();
    let day_start = clock.unix_timestamp - clock.unix_timestamp.rem_euclid(86_400);
    clock.unix_timestamp = day_start + hour * 3600 + minute * 60;
    env.svm.set_sysvar::<solana_clock::Clock>(&clock);
}
