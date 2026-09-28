use anchor_lang::prelude::*;
use mpl_core::types::OracleValidation;

// The account the Core program reads when it validates a transfer.
//
// Careful: this struct has to be EXACTLY an OracleValidation after the 8 byte
// anchor discriminator, because we register the oracle with
// ValidationResultsOffset::Anchor, which tells core to skip 8 bytes and read
// an OracleValidation from there. So no extra fields may be added here.
#[account]
pub struct OracleState {
    pub validation: OracleValidation,
}

impl OracleState {
    // Uninitialized + V1 with 4 results, each a 1 byte enum
    pub const SIZE: usize = 8 + 1 + 4;
}
