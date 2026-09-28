use anchor_lang::prelude::*;

// one config per collection. it holds the reward settings and is also the
// authority of the reward mint, so the program can mint rewards
#[account]
#[derive(InitSpace)]
pub struct StakeConfig {
    pub authority: Pubkey,   // admin, can change the settings
    pub collection: Pubkey,  // the core collection we are staking
    pub reward_mint: Pubkey, // the token users earn
    pub points_per_day: u64, // how many reward tokens per day per staked nft
    pub burn_bonus: u64,     // one time bonus for burning a staked nft
    pub bump: u8,
    pub reward_mint_bump: u8,
}

// one of these per staked nft. it is also the freeze delegate of the asset,
// so only this pda can unfreeze it
#[account]
#[derive(InitSpace)]
pub struct StakeAccount {
    pub owner: Pubkey,    // who staked it (they keep owning the nft)
    pub asset: Pubkey,    // the core asset
    pub staked_at: i64,   // when it was staked
    pub last_claim: i64,  // last time rewards were paid out
    pub bump: u8,
}

impl StakeAccount {
    // rewards that piled up since the last claim
    pub fn rewards_owed(&self, now: i64, points_per_day: u64) -> u64 {
        let elapsed = now.saturating_sub(self.last_claim).max(0) as u128;
        let per_day = points_per_day as u128;
        // elapsed seconds * tokens per day / seconds in a day
        let owed = elapsed * per_day / crate::constants::SECONDS_PER_DAY as u128;
        // can't realistically overflow a u64 but clamp anyway
        u64::try_from(owed).unwrap_or(u64::MAX)
    }
}
