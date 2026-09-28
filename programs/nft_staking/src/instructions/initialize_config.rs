use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token};

use crate::{constants::*, state::StakeConfig};

#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK: the core collection. we only store its address here, the core
    /// program checks it properly on every stake
    pub collection: UncheckedAccount<'info>,

    #[account(
        init,
        payer = authority,
        seeds = [CONFIG_SEED, collection.key().as_ref()],
        bump,
        space = 8 + StakeConfig::INIT_SPACE,
    )]
    pub config: Account<'info, StakeConfig>,

    // the reward token. the config pda is the mint authority so the program
    // can mint rewards without anyone holding a key
    #[account(
        init,
        payer = authority,
        seeds = [REWARDS_SEED, config.key().as_ref()],
        bump,
        mint::decimals = REWARD_DECIMALS,
        mint::authority = config,
    )]
    pub reward_mint: Account<'info, Mint>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> InitializeConfig<'info> {
    pub fn handler(
        &mut self,
        points_per_day: u64,
        burn_bonus: u64,
        bumps: &InitializeConfigBumps,
    ) -> Result<()> {
        self.config.set_inner(StakeConfig {
            authority: self.authority.key(),
            collection: self.collection.key(),
            reward_mint: self.reward_mint.key(),
            points_per_day,
            burn_bonus,
            bump: bumps.config,
            reward_mint_bump: bumps.reward_mint,
        });

        Ok(())
    }
}
