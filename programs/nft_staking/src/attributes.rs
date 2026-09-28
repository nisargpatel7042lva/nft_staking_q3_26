use anchor_lang::prelude::*;
use mpl_core::{
    fetch_collection_plugin,
    instructions::{
        UpdateCollectionPluginV1Cpi, UpdateCollectionPluginV1CpiAccounts,
        UpdateCollectionPluginV1InstructionArgs,
    },
    types::{Attribute, Attributes, Plugin, PluginType},
};

use crate::{constants::CONFIG_SEED, error::StakeError, state::StakeConfig};

// the attribute key we keep the counter under
pub const TOTAL_STAKED: &str = "total_staked";

// Task 1: the collection carries a "total_staked" attribute that goes up on
// every stake and down on every unstake / burn. The Attributes plugin is put
// on the collection when the config is created, and its authority is the
// config pda, so only this program can move the number.

// reads the counter off the collection. missing or unparseable counts as 0
pub fn read_total_staked(collection: &AccountInfo) -> Result<(u64, Vec<Attribute>)> {
    let (_, attributes, _) =
        fetch_collection_plugin::<Attributes>(collection, PluginType::Attributes)
            .map_err(|_| error!(StakeError::MissingAttributes))?;

    let total = attributes
        .attribute_list
        .iter()
        .find(|a| a.key == TOTAL_STAKED)
        .and_then(|a| a.value.parse::<u64>().ok())
        .unwrap_or(0);

    Ok((total, attributes.attribute_list))
}

// writes the counter back, keeping any other attributes the collection has
fn write_total_staked<'info>(
    collection: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    config: &Account<'info, StakeConfig>,
    system_program: &AccountInfo<'info>,
    mpl_core_program: &AccountInfo<'info>,
    mut attribute_list: Vec<Attribute>,
    total: u64,
) -> Result<()> {
    match attribute_list.iter_mut().find(|a| a.key == TOTAL_STAKED) {
        Some(existing) => existing.value = total.to_string(),
        None => attribute_list.push(Attribute {
            key: TOTAL_STAKED.to_string(),
            value: total.to_string(),
        }),
    }

    let collection_key = config.collection;
    let signer_seeds: &[&[&[u8]]] = &[&[CONFIG_SEED, collection_key.as_ref(), &[config.bump]]];

    UpdateCollectionPluginV1Cpi::new(
        mpl_core_program,
        UpdateCollectionPluginV1CpiAccounts {
            collection,
            payer,
            authority: Some(&config.to_account_info()),
            system_program,
            log_wrapper: None,
        },
        UpdateCollectionPluginV1InstructionArgs {
            plugin: Plugin::Attributes(Attributes { attribute_list }),
        },
    )
    .invoke_signed(signer_seeds)?;

    Ok(())
}

// +1 when an nft is staked
pub fn increment_total_staked<'info>(
    collection: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    config: &Account<'info, StakeConfig>,
    system_program: &AccountInfo<'info>,
    mpl_core_program: &AccountInfo<'info>,
) -> Result<()> {
    let (total, list) = read_total_staked(collection)?;
    let next = total.checked_add(1).ok_or(StakeError::Overflow)?;
    write_total_staked(
        collection,
        payer,
        config,
        system_program,
        mpl_core_program,
        list,
        next,
    )
}

// -1 when it is unstaked or burned. saturates so it can never wrap around
pub fn decrement_total_staked<'info>(
    collection: &AccountInfo<'info>,
    payer: &AccountInfo<'info>,
    config: &Account<'info, StakeConfig>,
    system_program: &AccountInfo<'info>,
    mpl_core_program: &AccountInfo<'info>,
) -> Result<()> {
    let (total, list) = read_total_staked(collection)?;
    let next = total.saturating_sub(1);
    write_total_staked(
        collection,
        payer,
        config,
        system_program,
        mpl_core_program,
        list,
        next,
    )
}
