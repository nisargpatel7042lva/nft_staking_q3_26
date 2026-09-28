use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token};
use mpl_core::{
    accounts::BaseCollectionV1,
    instructions::{
        AddCollectionPluginV1Cpi, AddCollectionPluginV1CpiAccounts,
        AddCollectionPluginV1InstructionArgs,
    },
    types::{Attribute, Attributes, Plugin, PluginAuthority},
    ID as MPL_CORE_ID,
};

use crate::{attributes::TOTAL_STAKED, constants::*, error::StakeError, state::StakeConfig};

#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    // the collection we are going to stake. the admin has to be its update
    // authority, otherwise core will not let us put the Attributes plugin on it
    #[account(
        mut,
        constraint = collection.update_authority == authority.key() @ StakeError::NotTheOwner,
    )]
    pub collection: Account<'info, BaseCollectionV1>,

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

    #[account(address = MPL_CORE_ID)]
    /// CHECK: checked by the address constraint
    pub mpl_core_program: UncheckedAccount<'info>,
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

        // Put the staking counter on the collection itself. The authority of
        // the plugin is the config pda, so from here on only this program can
        // change the number, not even the admin can fake it by hand.
        AddCollectionPluginV1Cpi::new(
            &self.mpl_core_program.to_account_info(),
            AddCollectionPluginV1CpiAccounts {
                collection: &self.collection.to_account_info(),
                payer: &self.authority.to_account_info(),
                authority: Some(&self.authority.to_account_info()),
                system_program: &self.system_program.to_account_info(),
                log_wrapper: None,
            },
            AddCollectionPluginV1InstructionArgs {
                plugin: Plugin::Attributes(Attributes {
                    attribute_list: vec![Attribute {
                        key: TOTAL_STAKED.to_string(),
                        value: "0".to_string(),
                    }],
                }),
                init_authority: Some(PluginAuthority::Address {
                    address: self.config.key(),
                }),
            },
        )
        .invoke()?;

        Ok(())
    }
}
