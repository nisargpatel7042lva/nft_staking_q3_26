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

// seeds for the oracle pieces
#[constant]
pub const ORACLE_SEED: &[u8] = b"oracle";
#[constant]
pub const VAULT_SEED: &[u8] = b"vault";

// transfers are only allowed between these hours (UTC)
pub const OPEN_HOUR: i64 = 9;
pub const CLOSE_HOUR: i64 = 17;

// the crank is only paid if it runs within this many seconds of an
// open/close boundary, so nobody gets paid for spamming it all day
pub const CRANK_WINDOW_SECONDS: i64 = 15 * 60;

// how much the cranker gets
pub const CRANK_REWARD_LAMPORTS: u64 = 1_000_000; // 0.001 SOL
