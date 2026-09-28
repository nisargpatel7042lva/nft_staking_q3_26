use anchor_lang::prelude::*;

#[error_code]
pub enum StakeError {
    #[msg("You are not the owner of this NFT")]
    NotTheOwner,
    #[msg("This NFT belongs to a different collection")]
    WrongCollection,
    #[msg("Math overflow")]
    Overflow,
    #[msg("Nothing to claim yet")]
    NothingToClaim,
}
