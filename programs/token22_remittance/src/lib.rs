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

    pub fn configure_confidential_account(
        ctx: Context<ConfigureConfidentialAccount>,
        decryptable_zero_balance: [u8; 36],
        maximum_pending_balance_credit_counter: u64,
    ) -> Result<()> {
        ctx.accounts.configure(decryptable_zero_balance, maximum_pending_balance_credit_counter)
    }

    pub fn approve_confidential_account(
        ctx: Context<ApproveConfidentialAccount>,
    ) -> Result<()> {
        ctx.accounts.approve()
    }

    pub fn deposit_confidential_tokens(
        ctx: Context<DepositConfidentialTokens>,
        amount: u64,
        decimals: u8,
    ) -> Result<()> {
        ctx.accounts.deposit(amount, decimals)
    }

    pub fn apply_pending_balance(
        ctx: Context<ApplyPendingBalance>,
        expected_pending_balance_credit_counter: u64,
        new_decryptable_available_balance: [u8; 36],
    ) -> Result<()> {
        ctx.accounts.apply(expected_pending_balance_credit_counter, new_decryptable_available_balance)
    }

    pub fn confidential_transfer(
        ctx: Context<ConfidentialTransfer>,
        new_source_decryptable_available_balance: [u8; 36],
        transfer_amount_auditor_ciphertext_lo: [u8; 64],
        transfer_amount_auditor_ciphertext_hi: [u8; 64],
    ) -> Result<()> {
        ctx.accounts.transfer(
            new_source_decryptable_available_balance,
            transfer_amount_auditor_ciphertext_lo,
            transfer_amount_auditor_ciphertext_hi,
        )
    }

    pub fn withdraw_confidential_tokens(
        ctx: Context<WithdrawConfidentialTokens>,
        amount: u64,
        decimals: u8,
        new_decryptable_available_balance: [u8; 36],
        apply_pending_balance_first: bool,
        expected_pending_balance_credit_counter: u64,
        intermediate_decryptable_available_balance: Option<[u8; 36]>,
    ) -> Result<()> {
        ctx.accounts.withdraw(
            amount,
            decimals,
            new_decryptable_available_balance,
            apply_pending_balance_first,
            expected_pending_balance_credit_counter,
            intermediate_decryptable_available_balance,
        )
    }
}
