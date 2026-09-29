use anchor_lang::{prelude::*, system_program};
use anchor_spl::token_2022::Token2022;
use spl_token_2022_interface::{
    extension::{
        default_account_state::instruction::initialize_default_account_state,
        metadata_pointer::instruction::initialize as initialize_metadata_pointer,
        transfer_fee::instruction::initialize_transfer_fee_config,
        ExtensionType,
    },
    instruction::{initialize_mint2, initialize_mint_close_authority},
    state::{AccountState, Mint},
};
use crate::{constants::MAX_FEE_BASIS_POINTS, error::RemittanceError};

#[derive(Accounts)]
pub struct CreateRemittanceMint<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(mut)]
    pub mint: Signer<'info>,

    /// CHECK: Mint authority for newly issued tokens
    pub mint_authority: UncheckedAccount<'info>,

    /// CHECK: Freeze authority used to unfreeze individual accounts after KYC
    pub freeze_authority: UncheckedAccount<'info>,

    /// CHECK: Authority allowed to close the mint upon decommission
    pub close_authority: UncheckedAccount<'info>,

    /// CHECK: Authority allowed to update transfer fee configuration
    pub transfer_fee_config_authority: UncheckedAccount<'info>,

    /// CHECK: Authority allowed to withdraw withheld transfer fees
    pub withdraw_withheld_authority: UncheckedAccount<'info>,

    /// CHECK: Authority allowed to update metadata pointer
    pub metadata_pointer_authority: UncheckedAccount<'info>,

    pub token_2022_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

impl<'info> CreateRemittanceMint<'info> {
    pub fn create_mint(
        &mut self,
        transfer_fee_basis_points: u16,
        maximum_fee: u64,
        decimals: u8,
    ) -> Result<()> {
        require!(
            transfer_fee_basis_points <= MAX_FEE_BASIS_POINTS,
            RemittanceError::InvalidFeeRate
        );

        // 1. ExtensionType::try_calculate_account_len correctly sizing the mint
        let extension_types = [
            ExtensionType::TransferFeeConfig,
            ExtensionType::MetadataPointer,
            ExtensionType::DefaultAccountState,
            ExtensionType::MintCloseAuthority,
        ];

        let space = ExtensionType::try_calculate_account_len::<Mint>(&extension_types)
            .map_err(|_| RemittanceError::InvalidExtension)?;

        let lamports = Rent::get()?.minimum_balance(space);

        // Create the uninitialized mint account
        system_program::create_account(
            CpiContext::new(
                self.system_program.key(),
                system_program::CreateAccount {
                    from: self.payer.to_account_info(),
                    to: self.mint.to_account_info(),
                },
            ),
            lamports,
            space as u64,
            self.token_2022_program.key,
        )?;

        // 2. Initialize TransferFeeConfig extension
        let init_transfer_fee_ix = initialize_transfer_fee_config(
            self.token_2022_program.key,
            self.mint.key,
            Some(self.transfer_fee_config_authority.key),
            Some(self.withdraw_withheld_authority.key),
            transfer_fee_basis_points,
            maximum_fee,
        )?;
        anchor_lang::solana_program::program::invoke(
            &init_transfer_fee_ix,
            &[self.mint.to_account_info()],
        )?;

        // 3. Initialize MetadataPointer extension (pointed at the mint itself)
        let init_metadata_pointer_ix = initialize_metadata_pointer(
            self.token_2022_program.key,
            self.mint.key,
            Some(*self.metadata_pointer_authority.key),
            Some(*self.mint.key), // Pointed at the mint itself
        )?;
        anchor_lang::solana_program::program::invoke(
            &init_metadata_pointer_ix,
            &[self.mint.to_account_info()],
        )?;

        // 4. Initialize DefaultAccountState extension (Frozen)
        let init_default_state_ix = initialize_default_account_state(
            self.token_2022_program.key,
            self.mint.key,
            &AccountState::Frozen,
        )?;
        anchor_lang::solana_program::program::invoke(
            &init_default_state_ix,
            &[self.mint.to_account_info()],
        )?;

        // 5. Initialize MintCloseAuthority extension
        let init_close_authority_ix = initialize_mint_close_authority(
            self.token_2022_program.key,
            self.mint.key,
            Some(self.close_authority.key),
        )?;
        anchor_lang::solana_program::program::invoke(
            &init_close_authority_ix,
            &[self.mint.to_account_info()],
        )?;

        // 6. InitializeMint2 (MUST be ordered after all extension-init instructions!)
        let init_mint_ix = initialize_mint2(
            self.token_2022_program.key,
            self.mint.key,
            self.mint_authority.key,
            Some(self.freeze_authority.key),
            decimals,
        )?;
        anchor_lang::solana_program::program::invoke(
            &init_mint_ix,
            &[self.mint.to_account_info()],
        )?;

        Ok(())
    }
}
