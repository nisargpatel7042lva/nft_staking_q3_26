use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};
use mpl_core::{
    accounts::{BaseAssetV1, BaseCollectionV1},
    instructions::{
        RemovePluginV1Cpi, RemovePluginV1CpiAccounts, RemovePluginV1InstructionArgs,
        UpdatePluginV1Cpi, UpdatePluginV1CpiAccounts, UpdatePluginV1InstructionArgs,
    },
    types::{FreezeDelegate, Plugin, PluginType},
    ID as MPL_CORE_ID,
};

use crate::{
    constants::*,
    error::StakeError,
    rewards::mint_rewards,
    state::{StakeAccount, StakeConfig},
};

#[derive(Accounts)]
pub struct Unstake<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(mut)]
    pub asset: Account<'info, BaseAssetV1>,

    #[account(mut)]
    pub collection: Account<'info, BaseCollectionV1>,

    #[account(
        seeds = [CONFIG_SEED, collection.key().as_ref()],
        bump = config.bump,
        has_one = collection,
        has_one = reward_mint,
    )]
    pub config: Account<'info, StakeConfig>,

    // closing the stake account gives the rent back to the owner
    #[account(
        mut,
        close = owner,
        seeds = [STAKE_SEED, asset.key().as_ref()],
        bump = stake_account.bump,
        has_one = owner @ StakeError::NotTheOwner,
        constraint = stake_account.asset == asset.key() @ StakeError::NotTheOwner,
    )]
    pub stake_account: Account<'info, StakeAccount>,

    #[account(
        mut,
        seeds = [REWARDS_SEED, config.key().as_ref()],
        bump = config.reward_mint_bump,
    )]
    pub reward_mint: Account<'info, Mint>,

    #[account(
        init_if_needed,
        payer = owner,
        associated_token::mint = reward_mint,
        associated_token::authority = owner,
    )]
    pub owner_rewards: Account<'info, TokenAccount>,

    #[account(address = MPL_CORE_ID)]
    /// CHECK: checked by the address constraint
    pub mpl_core_program: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> Unstake<'info> {
    pub fn handler(&mut self) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;

        // pay out whatever piled up since the last claim
        let owed = self
            .stake_account
            .rewards_owed(now, self.config.points_per_day);
        mint_rewards(
            &self.config,
            &self.reward_mint,
            &self.owner_rewards,
            &self.token_program,
            owed,
        )?;

        // the stake pda is the freeze delegate, so it has to sign these two
        let asset_key = self.asset.key();
        let signer_seeds: &[&[&[u8]]] =
            &[&[STAKE_SEED, asset_key.as_ref(), &[self.stake_account.bump]]];

        // unfreeze ...
        UpdatePluginV1Cpi::new(
            &self.mpl_core_program.to_account_info(),
            UpdatePluginV1CpiAccounts {
                asset: &self.asset.to_account_info(),
                collection: Some(&self.collection.to_account_info()),
                payer: &self.owner.to_account_info(),
                authority: Some(&self.stake_account.to_account_info()),
                system_program: &self.system_program.to_account_info(),
                log_wrapper: None,
            },
            UpdatePluginV1InstructionArgs {
                plugin: Plugin::FreezeDelegate(FreezeDelegate { frozen: false }),
            },
        )
        .invoke_signed(signer_seeds)?;

        // ... and take the plugin off completely, so the nft is clean again.
        // Note this one is signed by the OWNER, not the stake pda: core only
        // lets the delegate flip the frozen flag, removing the plugin is the
        // owner's call (and it only works because we just unfroze it).
        RemovePluginV1Cpi::new(
            &self.mpl_core_program.to_account_info(),
            RemovePluginV1CpiAccounts {
                asset: &self.asset.to_account_info(),
                collection: Some(&self.collection.to_account_info()),
                payer: &self.owner.to_account_info(),
                authority: Some(&self.owner.to_account_info()),
                system_program: &self.system_program.to_account_info(),
                log_wrapper: None,
            },
            RemovePluginV1InstructionArgs {
                plugin_type: PluginType::FreezeDelegate,
            },
        )
        .invoke()?;

        // and the burn delegate we added when staking
        RemovePluginV1Cpi::new(
            &self.mpl_core_program.to_account_info(),
            RemovePluginV1CpiAccounts {
                asset: &self.asset.to_account_info(),
                collection: Some(&self.collection.to_account_info()),
                payer: &self.owner.to_account_info(),
                authority: Some(&self.owner.to_account_info()),
                system_program: &self.system_program.to_account_info(),
                log_wrapper: None,
            },
            RemovePluginV1InstructionArgs {
                plugin_type: PluginType::BurnDelegate,
            },
        )
        .invoke()?;

        Ok(())
    }

}
