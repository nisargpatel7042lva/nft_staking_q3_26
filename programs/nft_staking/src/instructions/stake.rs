use anchor_lang::prelude::*;
use mpl_core::{
    accounts::{BaseAssetV1, BaseCollectionV1},
    instructions::{AddPluginV1Cpi, AddPluginV1CpiAccounts, AddPluginV1InstructionArgs},
    types::{FreezeDelegate, Plugin, PluginAuthority},
    ID as MPL_CORE_ID,
};

use crate::{
    constants::*,
    error::StakeError,
    state::{StakeAccount, StakeConfig},
};

#[derive(Accounts)]
pub struct Stake<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        constraint = asset.owner == owner.key() @ StakeError::NotTheOwner,
        constraint = asset.update_authority == mpl_core::types::UpdateAuthority::Collection(collection.key()) @ StakeError::WrongCollection,
    )]
    pub asset: Account<'info, BaseAssetV1>,

    #[account(mut)]
    pub collection: Account<'info, BaseCollectionV1>,

    #[account(
        seeds = [CONFIG_SEED, collection.key().as_ref()],
        bump = config.bump,
        has_one = collection,
    )]
    pub config: Account<'info, StakeConfig>,

    // this pda is created per asset and becomes the freeze delegate
    #[account(
        init,
        payer = owner,
        seeds = [STAKE_SEED, asset.key().as_ref()],
        bump,
        space = 8 + StakeAccount::INIT_SPACE,
    )]
    pub stake_account: Account<'info, StakeAccount>,

    #[account(address = MPL_CORE_ID)]
    /// CHECK: checked by the address constraint
    pub mpl_core_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> Stake<'info> {
    pub fn handler(&mut self, bumps: &StakeBumps) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;

        self.stake_account.set_inner(StakeAccount {
            owner: self.owner.key(),
            asset: self.asset.key(),
            staked_at: now,
            last_claim: now,
            bump: bumps.stake_account,
        });

        // Freeze the nft. The owner signs this (it is their asset), but the
        // plugin authority is set to the stake pda, so from now on only this
        // program can unfreeze it. That is what keeps the nft locked while
        // the user still owns it.
        AddPluginV1Cpi::new(
            &self.mpl_core_program.to_account_info(),
            AddPluginV1CpiAccounts {
                asset: &self.asset.to_account_info(),
                collection: Some(&self.collection.to_account_info()),
                payer: &self.owner.to_account_info(),
                authority: Some(&self.owner.to_account_info()),
                system_program: &self.system_program.to_account_info(),
                log_wrapper: None,
            },
            AddPluginV1InstructionArgs {
                plugin: Plugin::FreezeDelegate(FreezeDelegate { frozen: true }),
                init_authority: Some(PluginAuthority::Address {
                    address: self.stake_account.key(),
                }),
            },
        )
        .invoke()?;

        Ok(())
    }
}
