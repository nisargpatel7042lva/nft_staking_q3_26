// Helpers for talking to the Metaplex Core program from the tests:
// creating a collection, creating an asset, and reading plugins back out.
#![allow(dead_code)]

use {
    super::*,
    anchor_lang::AnchorDeserialize,
    mpl_core::{
        accounts::{BaseAssetV1, BaseCollectionV1, PluginHeaderV1},
        DataBlob, PluginRegistryV1Safe,
        instructions::{CreateCollectionV2Builder, CreateV2Builder},
        types::{Plugin, PluginAuthority, PluginType},
    },
    solana_keypair::Keypair,
    solana_signer::Signer,
};

// creates a core collection owned by the admin. `plugins` lets a test put
// things like Attributes on it at creation time
pub fn create_collection(
    env: &mut Env,
    plugins: Vec<mpl_core::types::PluginAuthorityPair>,
) -> Keypair {
    let collection = Keypair::new();
    let admin = env.admin.insecure_clone();

    let ix = CreateCollectionV2Builder::new()
        .collection(to_core_key(collection.pubkey()))
        .update_authority(Some(to_core_key(admin.pubkey())))
        .payer(to_core_key(env.payer.pubkey()))
        .name("Staking Collection".to_string())
        .uri("https://example.com/collection.json".to_string())
        .plugins(plugins)
        .instruction();

    // update_authority is only an address on CreateCollectionV2, it does not sign
    send(env, &[to_svm_ix(ix)], &[&collection]).unwrap();
    collection
}

// mints an asset of that collection to `owner`
pub fn create_asset(env: &mut Env, collection: &SvmKey, owner: &SvmKey) -> Keypair {
    let asset = Keypair::new();
    let admin = env.admin.insecure_clone();

    let ix = CreateV2Builder::new()
        .asset(to_core_key(asset.pubkey()))
        .collection(Some(to_core_key(*collection)))
        .authority(Some(to_core_key(admin.pubkey())))
        .payer(to_core_key(env.payer.pubkey()))
        .owner(Some(to_core_key(*owner)))
        .name("Staked NFT".to_string())
        .uri("https://example.com/nft.json".to_string())
        .instruction();

    send(env, &[to_svm_ix(ix)], &[&asset, &admin]).unwrap();
    asset
}

// where the plugin registry lives depends on the base account size, so we do
// it separately for assets and collections
fn plugin_records(data: &[u8], base_len: usize) -> Vec<(PluginType, PluginAuthority, usize)> {
    if data.len() <= base_len {
        return vec![]; // no plugins at all
    }
    let header = match PluginHeaderV1::from_bytes(&data[base_len..]) {
        Ok(h) => h,
        Err(_) => return vec![],
    };
    let registry =
        match PluginRegistryV1Safe::from_bytes(&data[header.plugin_registry_offset as usize..]) {
            Ok(r) => r,
            Err(_) => return vec![],
        };

    registry
        .registry
        .iter()
        .filter_map(|record| {
            let plugin_type = plugin_type_from_u8(record.plugin_type)?;
            Some((plugin_type, record.authority.clone(), record.offset as usize))
        })
        .collect()
}

// the safe registry gives us a raw u8, turn it back into the enum
fn plugin_type_from_u8(value: u8) -> Option<PluginType> {
    // borsh encodes a fieldless enum as a single byte, so round tripping works
    PluginType::deserialize(&mut &[value][..]).ok()
}

fn asset_base_len(data: &[u8]) -> usize {
    BaseAssetV1::from_bytes(data).map(|a| a.len()).unwrap_or(0)
}

fn collection_base_len(data: &[u8]) -> usize {
    BaseCollectionV1::from_bytes(data)
        .map(|c| c.len())
        .unwrap_or(0)
}

// reads one plugin off an asset, together with who controls it
pub fn asset_plugin(env: &Env, asset: &SvmKey, plugin_type: PluginType) -> Option<(PluginAuthority, Plugin)> {
    let data = account_data(env, asset);
    read_plugin(&data, asset_base_len(&data), plugin_type)
}

pub fn collection_plugin(
    env: &Env,
    collection: &SvmKey,
    plugin_type: PluginType,
) -> Option<(PluginAuthority, Plugin)> {
    let data = account_data(env, collection);
    read_plugin(&data, collection_base_len(&data), plugin_type)
}

fn read_plugin(data: &[u8], base_len: usize, plugin_type: PluginType) -> Option<(PluginAuthority, Plugin)> {
    let (_, authority, offset) = plugin_records(data, base_len)
        .into_iter()
        .find(|(t, _, _)| *t == plugin_type)?;
    let plugin = Plugin::deserialize(&mut &data[offset..]).ok()?;
    Some((authority, plugin))
}

pub fn has_plugin(env: &Env, key: &SvmKey, plugin_type: PluginType, is_collection: bool) -> bool {
    if is_collection {
        collection_plugin(env, key, plugin_type).is_some()
    } else {
        asset_plugin(env, key, plugin_type).is_some()
    }
}

// true if the asset is frozen by a FreezeDelegate plugin
pub fn is_frozen(env: &Env, asset: &SvmKey) -> bool {
    match asset_plugin(env, asset, PluginType::FreezeDelegate) {
        Some((_, Plugin::FreezeDelegate(f))) => f.frozen,
        _ => false,
    }
}

pub fn asset_owner(env: &Env, asset: &SvmKey) -> SvmKey {
    let data = account_data(env, asset);
    to_svm_key(BaseAssetV1::from_bytes(&data).unwrap().owner)
}
