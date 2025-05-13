use anchor_lang::prelude::*;

#[error_code]
pub enum CustomError {
    #[msg("Wrong Influencer")]
    InvalidInfluencer,
    #[msg("Wrong Owner Account")]
    InvalidAccountOwner,
    #[msg("Price should not be negative")]
    InvalidPrice,
}