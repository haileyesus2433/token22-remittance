pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
#[allow(unused_imports)]
pub use state::*;

declare_id!("9GbUc5TpkPjpUfBq477LecNieSVtREA7jxaxxEJttrb5");

#[program]
pub mod token22_remittance {
    use super::*;

    pub fn create_remittance_mint(
        ctx: Context<CreateRemittanceMint>,
        transfer_fee_basis_points: u16,
        maximum_fee: u64,
        decimals: u8,
    ) -> Result<()> {
        ctx.accounts.create_mint(transfer_fee_basis_points, maximum_fee, decimals)
    }

    pub fn transfer_with_fee(
        ctx: Context<TransferRemittanceWithFee>,
        amount: u64,
    ) -> Result<()> {
        ctx.accounts.transfer_with_fee(amount)
    }

    pub fn thaw_kyc_account(ctx: Context<ThawKycAccount>) -> Result<()> {
        ctx.accounts.thaw()
    }

    pub fn reissue_remittance_mint(
        ctx: Context<ReissueRemittanceMint>,
        transfer_fee_basis_points: u16,
        maximum_fee: u64,
        decimals: u8,
    ) -> Result<()> {
        ctx.accounts.reissue_mint(transfer_fee_basis_points, maximum_fee, decimals)
    }
}
