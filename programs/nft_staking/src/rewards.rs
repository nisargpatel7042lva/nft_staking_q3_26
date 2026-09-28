use anchor_lang::prelude::*;
use anchor_spl::token::{mint_to, Mint, MintTo, Token, TokenAccount};

use crate::{constants::CONFIG_SEED, state::StakeConfig};

// Mints reward tokens to a user. The config pda is the mint authority, so it
// has to sign. Used by unstake, claim_rewards and burn_staked_nft.
pub fn mint_rewards<'info>(
    config: &Account<'info, StakeConfig>,
    reward_mint: &Account<'info, Mint>,
    destination: &Account<'info, TokenAccount>,
    token_program: &Program<'info, Token>,
    amount: u64,
) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }

    let collection = config.collection;
    let signer_seeds: &[&[&[u8]]] = &[&[CONFIG_SEED, collection.as_ref(), &[config.bump]]];

    mint_to(
        CpiContext::new_with_signer(
            token_program.to_account_info(),
            MintTo {
                mint: reward_mint.to_account_info(),
                to: destination.to_account_info(),
                authority: config.to_account_info(),
            },
            signer_seeds,
        ),
        amount,
    )
}
