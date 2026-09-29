use anchor_lang::prelude::*;
use anchor_spl::token_2022::Token2022;
use spl_token_2022_interface::extension::transfer_fee::instruction::transfer_checked_with_fee;
use crate::{error::RemittanceError, state::StateReader};

#[derive(Accounts)]
pub struct TransferRemittanceWithFee<'info> {
    pub authority: Signer<'info>,

    #[account(mut)]
    /// CHECK: Validated via StateWithExtensions and Token-2022 CPI
    pub from: UncheckedAccount<'info>,

    /// CHECK: Mint account validated via StateWithExtensions
    pub mint: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Destination token account validated via StateWithExtensions and Token-2022 CPI
    pub to: UncheckedAccount<'info>,

    pub token_2022_program: Program<'info, Token2022>,
}

impl<'info> TransferRemittanceWithFee<'info> {
    pub fn transfer_with_fee(&self, amount: u64) -> Result<()> {
        // Task 3: Read account/mint state exclusively through StateWithExtensions, never raw unpack
        let mint_data = self.mint.try_borrow_data()?;
        let mint_state = StateReader::unpack_mint(&mint_data)?;

        let from_data = self.from.try_borrow_data()?;
        let from_state = StateReader::unpack_account(&from_data)?;

        let to_data = self.to.try_borrow_data()?;
        let to_state = StateReader::unpack_account(&to_data)?;

        // Safety checks on accounts
        require_keys_eq!(from_state.base.mint, *self.mint.key, RemittanceError::InvalidMint);
        require_keys_eq!(to_state.base.mint, *self.mint.key, RemittanceError::InvalidMint);
        require_keys_eq!(from_state.base.owner, *self.authority.key, RemittanceError::Unauthorized);

        // Task 2: dynamic fee via calculate_epoch_fee(current_epoch, amount) rather than cached rate
        let current_epoch = Clock::get()?.epoch;
        let fee_config = StateReader::get_transfer_fee_config(&mint_state)?;

        let fee = fee_config
            .calculate_epoch_fee(current_epoch, amount)
            .ok_or(RemittanceError::FeeCalculationOverflow)?;

        let decimals = mint_state.base.decimals;

        // Drop borrows before CPI invocation
        drop(mint_data);
        drop(from_data);
        drop(to_data);

        // transfer_checked_with_fee instruction
        let ix = transfer_checked_with_fee(
            self.token_2022_program.key,
            self.from.key,
            self.mint.key,
            self.to.key,
            self.authority.key,
            &[],
            amount,
            decimals,
            fee,
        )?;

        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                self.from.to_account_info(),
                self.mint.to_account_info(),
                self.to.to_account_info(),
                self.authority.to_account_info(),
            ],
        )?;

        Ok(())
    }
}
