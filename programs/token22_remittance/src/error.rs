use anchor_lang::prelude::*;

#[error_code]
pub enum RemittanceError {
    #[msg("Invalid transfer fee rate")]
    InvalidFeeRate,
    #[msg("Account is frozen / pending KYC verification")]
    AccountFrozen,
    #[msg("Calculation overflow")]
    MathOverflow,
    #[msg("Unauthorized signer")]
    Unauthorized,
    #[msg("Invalid extension configuration")]
    InvalidExtension,
}
