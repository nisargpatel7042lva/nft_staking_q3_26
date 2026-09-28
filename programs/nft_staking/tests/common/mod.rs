// Shared test setup.
//
// One annoying detail: anchor 0.32 and mpl-core are built against the solana
// 2.x crates, while litesvm uses the 3.x/4.x ones. Pubkey and Instruction are
// the same bytes on both sides, they are just different Rust types, so the
// little `to_svm_*` / `to_core_*` helpers below convert between them.
#![allow(dead_code)]

pub mod core;

use {
    anchor_lang::{InstructionData, ToAccountMetas},
    litesvm::{types::TransactionResult, LiteSVM},
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

// types as anchor / mpl-core see them
pub type CoreKey = anchor_lang::prelude::Pubkey;
pub type CoreIx = anchor_lang::solana_program::instruction::Instruction;
// types as litesvm sees them
pub type SvmKey = solana_pubkey::Pubkey;
pub type SvmIx = solana_instruction::Instruction;

pub fn to_svm_key(key: CoreKey) -> SvmKey {
    SvmKey::new_from_array(key.to_bytes())
}

pub fn to_core_key(key: SvmKey) -> CoreKey {
    CoreKey::new_from_array(key.to_bytes())
}

pub fn to_svm_ix(ix: CoreIx) -> SvmIx {
    SvmIx {
        program_id: to_svm_key(ix.program_id),
        accounts: ix
            .accounts
            .into_iter()
            .map(|a| solana_instruction::AccountMeta {
                pubkey: to_svm_key(a.pubkey),
                is_signer: a.is_signer,
                is_writable: a.is_writable,
            })
            .collect(),
        data: ix.data,
    }
}

// the two programs we need in the test validator
const MPL_CORE: &[u8] = include_bytes!("../../../../tests/fixtures/mpl_core.so");
const NFT_STAKING: &[u8] = include_bytes!("../../../../target/deploy/nft_staking.so");

pub const POINTS_PER_DAY: u64 = 10_000_000; // 10 reward tokens per day
pub const BURN_BONUS: u64 = 500_000_000; // 500 reward tokens for burning
pub const DAY: i64 = 86_400;

pub struct Env {
    pub svm: LiteSVM,
    pub payer: Keypair,
    pub admin: Keypair,
}

pub fn setup() -> Env {
    let mut svm = LiteSVM::new();
    svm.add_program(to_svm_key(mpl_core::ID), MPL_CORE).unwrap();
    svm.add_program(to_svm_key(nft_staking::ID), NFT_STAKING)
        .unwrap();

    let payer = Keypair::new();
    let admin = Keypair::new();
    svm.airdrop(&payer.pubkey(), 100_000_000_000).unwrap();
    svm.airdrop(&admin.pubkey(), 100_000_000_000).unwrap();

    Env { svm, payer, admin }
}

// sends a transaction, the payer always signs
pub fn send(env: &mut Env, ixs: &[SvmIx], extra_signers: &[&Keypair]) -> TransactionResult {
    let payer = env.payer.insecure_clone();
    send_with_payer(env, ixs, &payer, extra_signers)
}

pub fn send_with_payer(
    env: &mut Env,
    ixs: &[SvmIx],
    payer: &Keypair,
    extra_signers: &[&Keypair],
) -> TransactionResult {
    env.svm.expire_blockhash();
    let blockhash = env.svm.latest_blockhash();
    let msg = Message::new_with_blockhash(ixs, Some(&payer.pubkey()), &blockhash);

    let mut signers: Vec<&Keypair> = vec![payer];
    for s in extra_signers {
        if !signers.iter().any(|k| k.pubkey() == s.pubkey()) {
            signers.push(s);
        }
    }

    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &signers).unwrap();
    env.svm.send_transaction(tx)
}

// builds one of our program's instructions from the anchor generated types
pub fn program_ix<A: ToAccountMetas, D: InstructionData>(accounts: A, data: D) -> SvmIx {
    to_svm_ix(CoreIx {
        program_id: nft_staking::ID,
        accounts: accounts.to_account_metas(None),
        data: data.data(),
    })
}

pub fn account_data(env: &Env, key: &SvmKey) -> Vec<u8> {
    env.svm.get_account(key).expect("account not found").data
}

pub fn exists(env: &Env, key: &SvmKey) -> bool {
    env.svm
        .get_account(key)
        .map(|a| a.lamports > 0)
        .unwrap_or(false)
}

// moves the clock forward so rewards accumulate
pub fn warp_seconds(env: &mut Env, seconds: i64) {
    let mut clock = env.svm.get_sysvar::<solana_clock::Clock>();
    clock.unix_timestamp += seconds;
    env.svm.set_sysvar::<solana_clock::Clock>(&clock);
}

pub fn now(env: &Env) -> i64 {
    env.svm.get_sysvar::<solana_clock::Clock>().unix_timestamp
}

// checks a transaction failed with one of our anchor errors
pub fn assert_program_error(res: TransactionResult, code: u32) {
    let err = res.expect_err("transaction should have failed");
    let msg = format!("{:?}", err.err);
    assert!(
        msg.contains(&format!("Custom({})", code)),
        "expected error {} but got: {}",
        code,
        msg
    );
}

// reward token balance of an spl token account, 0 if it does not exist yet
pub fn token_balance(env: &Env, ata: &SvmKey) -> u64 {
    use litesvm_token::spl_token::state::Account as TokenAccount;
    match litesvm_token::get_spl_account::<TokenAccount>(&env.svm, ata) {
        Ok(account) => account.amount,
        Err(_) => 0,
    }
}

// the associated token account address for the reward mint
pub fn reward_ata(owner: &SvmKey, mint: &SvmKey) -> SvmKey {
    to_svm_key(anchor_spl::associated_token::get_associated_token_address(
        &to_core_key(*owner),
        &to_core_key(*mint),
    ))
}
