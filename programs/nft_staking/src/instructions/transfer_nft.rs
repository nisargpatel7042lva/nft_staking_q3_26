use anchor_lang::prelude::*;
use mpl_core::{
    accounts::{BaseAssetV1, BaseCollectionV1},
    instructions::{TransferV1Cpi, TransferV1CpiAccounts, TransferV1InstructionArgs},
    ID as MPL_CORE_ID,
};

use crate::{constants::*, error::StakeError, state_oracle::OracleState};

// Task 2: transferring an nft through our program.
//
// The interesting part is the oracle account: it is not one of core's normal
// accounts, so it goes in as a REMAINING account on the cpi. Core looks up the
// oracle registered on the collection, finds the account in the list, reads the
// transfer result out of it and rejects the whole thing if it says Rejected.
#[derive(Accounts)]
pub struct TransferNft<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        constraint = asset.owner == owner.key() @ StakeError::NotTheOwner,
    )]
    pub asset: Account<'info, BaseAssetV1>,

    #[account(mut)]
    pub collection: Account<'info, BaseCollectionV1>,

    /// CHECK: any wallet can receive the nft
    pub new_owner: UncheckedAccount<'info>,

    #[account(
        seeds = [ORACLE_SEED, collection.key().as_ref()],
        bump,
    )]
    pub oracle: Account<'info, OracleState>,

    #[account(address = MPL_CORE_ID)]
    /// CHECK: checked by the address constraint
    pub mpl_core_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> TransferNft<'info> {
    pub fn handler(&mut self) -> Result<()> {
        TransferV1Cpi::new(
            &self.mpl_core_program.to_account_info(),
            TransferV1CpiAccounts {
                asset: &self.asset.to_account_info(),
                collection: Some(&self.collection.to_account_info()),
                payer: &self.owner.to_account_info(),
                authority: Some(&self.owner.to_account_info()),
                new_owner: &self.new_owner.to_account_info(),
                system_program: Some(&self.system_program.to_account_info()),
                log_wrapper: None,
            },
            TransferV1InstructionArgs {
                compression_proof: None,
            },
        )
        // the oracle rides along as a remaining account, read only, no signature
        .invoke_with_remaining_accounts(&[(&self.oracle.to_account_info(), false, false)])?;

        Ok(())
    }
}
