use anchor_lang::prelude::*;
use mpl_core::{
    accounts::BaseCollectionV1,
    types::{ExternalValidationResult, OracleValidation},
};

use crate::{constants::*, state_oracle::OracleState};

// Task 2: the crank.
//
// Anyone may call this, there is no authority check. It reads the clock, works
// out whether we are inside trading hours and writes Approved or Rejected into
// the oracle account. Core reads that account on the next transfer.
//
// To stop people from farming the reward by calling this in a loop, the payout
// only happens when the call actually FLIPS the state and it happens close to
// an open/close boundary. Every other call still updates the state, it just
// does not pay.
#[derive(Accounts)]
pub struct UpdateOracle<'info> {
    // whoever runs the crank, they get the reward
    #[account(mut)]
    pub cranker: Signer<'info>,

    pub collection: Account<'info, BaseCollectionV1>,

    #[account(
        mut,
        seeds = [ORACLE_SEED, collection.key().as_ref()],
        bump,
    )]
    pub oracle: Account<'info, OracleState>,

    #[account(
        mut,
        seeds = [VAULT_SEED, collection.key().as_ref()],
        bump,
    )]
    pub vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}

// are we inside trading hours right now?
pub fn market_is_open(unix_timestamp: i64) -> bool {
    let seconds_into_day = unix_timestamp.rem_euclid(SECONDS_PER_DAY);
    let hour = seconds_into_day / 3600;
    hour >= OPEN_HOUR && hour < CLOSE_HOUR
}

// how far are we from the next open or close, in seconds
pub fn seconds_from_boundary(unix_timestamp: i64) -> i64 {
    let seconds_into_day = unix_timestamp.rem_euclid(SECONDS_PER_DAY);
    let open = OPEN_HOUR * 3600;
    let close = CLOSE_HOUR * 3600;

    // distance to today's open, today's close, and tomorrow's open
    [
        (seconds_into_day - open).abs(),
        (seconds_into_day - close).abs(),
        (seconds_into_day + SECONDS_PER_DAY - open).abs(),
        (SECONDS_PER_DAY - seconds_into_day + open).abs(),
    ]
    .into_iter()
    .min()
    .unwrap_or(i64::MAX)
}

impl<'info> UpdateOracle<'info> {
    pub fn handler(&mut self, collection_bump: u8) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;

        let should_allow = market_is_open(now);
        let new_transfer_state = if should_allow {
            ExternalValidationResult::Approved
        } else {
            ExternalValidationResult::Rejected
        };

        // what does the oracle say at the moment?
        let old_transfer_state = match &self.oracle.validation {
            OracleValidation::V1 { transfer, .. } => transfer.clone(),
            OracleValidation::Uninitialized => ExternalValidationResult::Rejected,
        };

        let flipped = old_transfer_state != new_transfer_state;

        self.oracle.validation = OracleValidation::V1 {
            create: ExternalValidationResult::Pass,
            transfer: new_transfer_state,
            burn: ExternalValidationResult::Pass,
            update: ExternalValidationResult::Pass,
        };

        // pay the cranker only if they did something useful: flipped the state,
        // and did it near the boundary rather than hours later
        let near_boundary = seconds_from_boundary(now) <= CRANK_WINDOW_SECONDS;
        if flipped && near_boundary {
            self.pay_cranker(collection_bump)?;
        }

        Ok(())
    }

    fn pay_cranker(&self, collection_bump: u8) -> Result<()> {
        let available = self.vault.lamports();
        // never drain the vault below its rent, and skip if it is empty
        let rent = Rent::get()?.minimum_balance(0);
        if available <= rent {
            return Ok(());
        }
        let payout = CRANK_REWARD_LAMPORTS.min(available - rent);
        if payout == 0 {
            return Ok(());
        }

        let collection_key = self.collection.key();
        let signer_seeds: &[&[&[u8]]] =
            &[&[VAULT_SEED, collection_key.as_ref(), &[collection_bump]]];

        anchor_lang::system_program::transfer(
            CpiContext::new_with_signer(
                self.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: self.vault.to_account_info(),
                    to: self.cranker.to_account_info(),
                },
                signer_seeds,
            ),
            payout,
        )
    }
}
