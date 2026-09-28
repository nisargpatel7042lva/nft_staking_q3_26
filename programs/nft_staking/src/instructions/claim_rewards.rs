use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};
use mpl_core::accounts::BaseAssetV1;

use crate::{
    constants::*,
    error::StakeError,
    rewards::mint_rewards,
    state::{StakeAccount, StakeConfig},
};

// Task 1: collect the rewards that piled up WITHOUT unstaking.
// The nft stays frozen and the stake account stays open, we only move the
// last_claim timestamp forward so the same seconds can't be claimed twice.
#[derive(Accounts)]
pub struct ClaimRewards<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    // not mut: we are not touching the nft at all here
    pub asset: Account<'info, BaseAssetV1>,

    #[account(
        seeds = [CONFIG_SEED, config.collection.as_ref()],
        bump = config.bump,
        has_one = reward_mint,
    )]
    pub config: Account<'info, StakeConfig>,

    #[account(
        mut,
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

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

impl<'info> ClaimRewards<'info> {
    pub fn handler(&mut self) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;

        let owed = self
            .stake_account
            .rewards_owed(now, self.config.points_per_day);
        require!(owed > 0, StakeError::NothingToClaim);

        mint_rewards(
            &self.config,
            &self.reward_mint,
            &self.owner_rewards,
            &self.token_program,
            owed,
        )?;

        // start counting again from now. the nft is still frozen and still staked
        self.stake_account.last_claim = now;

        Ok(())
    }
}
