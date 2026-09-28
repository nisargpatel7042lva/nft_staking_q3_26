pub mod attributes;
pub mod constants;
pub mod error;
pub mod instructions;
pub mod rewards;
pub mod state;
pub mod state_oracle;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;
pub use state_oracle::*;

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

    // ---- task 2: the time based transfer oracle ----

    // create the oracle account and attach it to the collection
    pub fn initialize_oracle(ctx: Context<InitializeOracle>, vault_funding: u64) -> Result<()> {
        ctx.accounts.handler(vault_funding)
    }

    // permissionless crank: anyone can refresh the oracle, and gets paid for
    // doing it at the right moment
    pub fn update_oracle(ctx: Context<UpdateOracle>) -> Result<()> {
        let vault_bump = ctx.bumps.vault;
        ctx.accounts.handler(vault_bump)
    }

    // transfer an nft, which the oracle may block outside trading hours
    pub fn transfer_nft(ctx: Context<TransferNft>) -> Result<()> {
        ctx.accounts.handler()
    }
}
