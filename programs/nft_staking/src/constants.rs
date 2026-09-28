use anchor_lang::prelude::*;

// seeds for all our pdas
#[constant]
pub const CONFIG_SEED: &[u8] = b"config";
#[constant]
pub const REWARDS_SEED: &[u8] = b"rewards";
#[constant]
pub const STAKE_SEED: &[u8] = b"stake";

// the reward token has 6 decimals like most spl tokens
pub const REWARD_DECIMALS: u8 = 6;

// seconds in a day, used for the reward math
pub const SECONDS_PER_DAY: i64 = 86_400;
