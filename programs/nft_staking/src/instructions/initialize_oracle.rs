use anchor_lang::prelude::*;
use mpl_core::{
    accounts::BaseCollectionV1,
    instructions::{
        AddCollectionExternalPluginAdapterV1Cpi,
        AddCollectionExternalPluginAdapterV1CpiAccounts,
        AddCollectionExternalPluginAdapterV1InstructionArgs,
    },
    types::{
        ExternalCheckResult, ExternalPluginAdapterInitInfo, ExternalValidationResult,
        HookableLifecycleEvent, OracleInitInfo, OracleValidation, PluginAuthority,
        ValidationResultsOffset,
    },
    ID as MPL_CORE_ID,
};

use crate::{constants::*, error::StakeError, state_oracle::OracleState};

// Task 2: set up the time based transfer oracle.
//
// Three things happen here:
//   1. the oracle account (a pda, so its address is fixed and core can find it)
//   2. a vault pda that holds the lamports we pay the cranker from
//   3. the oracle plugin adapter is attached to the collection, watching only
//      the Transfer lifecycle event and only allowed to REJECT
#[derive(Accounts)]
pub struct InitializeOracle<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(
        mut,
        constraint = collection.update_authority == authority.key() @ StakeError::NotTheOwner,
    )]
    pub collection: Account<'info, BaseCollectionV1>,

    #[account(
        init,
        payer = authority,
        seeds = [ORACLE_SEED, collection.key().as_ref()],
        bump,
        space = OracleState::SIZE,
    )]
    pub oracle: Account<'info, OracleState>,

    // plain lamport account, the crank rewards come out of here
    #[account(
        mut,
        seeds = [VAULT_SEED, collection.key().as_ref()],
        bump,
    )]
    pub vault: SystemAccount<'info>,

    #[account(address = MPL_CORE_ID)]
    /// CHECK: checked by the address constraint
    pub mpl_core_program: UncheckedAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> InitializeOracle<'info> {
    pub fn handler(&mut self, vault_funding: u64) -> Result<()> {
        // start out rejecting everything until the first crank runs
        self.oracle.validation = OracleValidation::V1 {
            create: ExternalValidationResult::Pass,
            transfer: ExternalValidationResult::Rejected,
            burn: ExternalValidationResult::Pass,
            update: ExternalValidationResult::Pass,
        };

        // fund the crank vault
        if vault_funding > 0 {
            let cpi = anchor_lang::system_program::Transfer {
                from: self.authority.to_account_info(),
                to: self.vault.to_account_info(),
            };
            anchor_lang::system_program::transfer(
                CpiContext::new(self.system_program.to_account_info(), cpi),
                vault_funding,
            )?;
        }

        // Hook the oracle up to the collection. lifecycle_checks says: only
        // look at Transfer, and the only thing you may do is reject it.
        // results_offset = Anchor means core skips our 8 byte discriminator.
        AddCollectionExternalPluginAdapterV1Cpi::new(
            &self.mpl_core_program.to_account_info(),
            AddCollectionExternalPluginAdapterV1CpiAccounts {
                collection: &self.collection.to_account_info(),
                payer: &self.authority.to_account_info(),
                authority: Some(&self.authority.to_account_info()),
                system_program: &self.system_program.to_account_info(),
                log_wrapper: None,
            },
            AddCollectionExternalPluginAdapterV1InstructionArgs {
                init_info: ExternalPluginAdapterInitInfo::Oracle(OracleInitInfo {
                    base_address: self.oracle.key(),
                    init_plugin_authority: Some(PluginAuthority::Address {
                        address: self.authority.key(),
                    }),
                    lifecycle_checks: vec![(
                        HookableLifecycleEvent::Transfer,
                        // bit 2 = "can reject", see ExternalCheckResultBits
                        ExternalCheckResult { flags: 4 },
                    )],
                    base_address_config: None,
                    results_offset: Some(ValidationResultsOffset::Anchor),
                }),
            },
        )
        .invoke()?;

        Ok(())
    }
}
