pub mod attributes;
pub mod constants;
pub mod error;
pub mod instructions;
pub mod rewards;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("5HFNNaejx1nHshjziR1pLReeMrTmBTBsCC4RQdwcy2mD");

#[program]
pub mod nft_staking {
    use super::*;

    // admin sets up the reward token and the reward rate for one collection
    pub fn initialize_config(
        ctx: Context<InitializeConfig>,
        points_per_day: u64,
        burn_bonus: u64,
    ) -> Result<()> {
        ctx.accounts
            .handler(points_per_day, burn_bonus, &ctx.bumps)
    }

    // lock an nft and start earning
    pub fn stake(ctx: Context<Stake>) -> Result<()> {
        ctx.accounts.handler(&ctx.bumps)
    }

    // unlock the nft and take the rewards with it
    pub fn unstake(ctx: Context<Unstake>) -> Result<()> {
        ctx.accounts.handler()
    }

    // take the rewards but leave the nft staked and frozen
    pub fn claim_rewards(ctx: Context<ClaimRewards>) -> Result<()> {
        ctx.accounts.handler()
    }

    // destroy the nft for a big one time bonus
    pub fn burn_staked_nft(ctx: Context<BurnStakedNft>) -> Result<()> {
        ctx.accounts.handler()
    }
}
