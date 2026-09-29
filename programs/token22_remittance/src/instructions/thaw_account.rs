use anchor_lang::prelude::*;
use anchor_spl::token_2022::Token2022;
use spl_token_2022_interface::{
    instruction::thaw_account,
    state::AccountState,
};
use crate::{
    error::RemittanceError,
    state::StateReader,
};

#[derive(Accounts)]
pub struct ThawKycAccount<'info> {
    pub freeze_authority: Signer<'info>,

    #[account(mut)]
    /// CHECK: Token account to be unfrozen after KYC, verified via StateReader
    pub account: UncheckedAccount<'info>,

    /// CHECK: Mint account holding freeze authority and DefaultAccountState extension
    pub mint: UncheckedAccount<'info>,

    pub token_2022_program: Program<'info, Token2022>,
}

impl<'info> ThawKycAccount<'info> {
    pub fn thaw(&self) -> Result<()> {
        // Read mint state via StateReader (Task 3)
        let mint_data = self.mint.try_borrow_data()?;
        let mint_state = StateReader::unpack_mint(&mint_data)?;

        // Verify freeze authority matches mint's configured freeze authority
        let expected_freeze_auth = mint_state
            .base
            .freeze_authority
            .ok_or(RemittanceError::Unauthorized)?;
        require_keys_eq!(
            expected_freeze_auth,
            *self.freeze_authority.key,
            RemittanceError::Unauthorized
        );

        // Read token account state via StateReader
        let acc_data = self.account.try_borrow_data()?;
        let acc_state = StateReader::unpack_account(&acc_data)?;

        // Safety checks: account belongs to this mint and is currently frozen
        require_keys_eq!(
            acc_state.base.mint,
            *self.mint.key,
            RemittanceError::InvalidMint
        );
        require!(
            acc_state.base.state == AccountState::Frozen,
            RemittanceError::AccountNotFrozen
        );

        // Drop borrows before invoking CPI
        drop(mint_data);
        drop(acc_data);

        // Invoke thaw_account CPI: freeze authority thaws this individual account
        // Notice: mint-level default state remains unchanged (DefaultAccountState stays Frozen)
        let ix = thaw_account(
            self.token_2022_program.key,
            self.account.key,
            self.mint.key,
            self.freeze_authority.key,
            &[],
        )?;

        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                self.account.to_account_info(),
                self.mint.to_account_info(),
                self.freeze_authority.to_account_info(),
            ],
        )?;

        Ok(())
    }
}
