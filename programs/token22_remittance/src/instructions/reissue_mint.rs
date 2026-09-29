use anchor_lang::{prelude::*, system_program};
use anchor_spl::token_2022::Token2022;
use spl_token_2022_interface::{
    extension::{
        confidential_transfer::instruction::initialize_mint as initialize_confidential_transfer_mint,
        confidential_transfer_fee::instruction::initialize_confidential_transfer_fee_config,
        default_account_state::instruction::initialize_default_account_state,
        metadata_pointer::instruction::initialize as initialize_metadata_pointer,
        transfer_fee::instruction::initialize_transfer_fee_config,
        ExtensionType,
    },
    instruction::{
        initialize_mint2,
        initialize_mint_close_authority,
        initialize_permanent_delegate,
    },
    solana_zk_sdk::encryption::pod::elgamal::PodElGamalPubkey,
    state::{AccountState, Mint},
};
use crate::{constants::MAX_FEE_BASIS_POINTS, error::RemittanceError};

#[derive(Accounts)]
pub struct ReissueRemittanceMint<'info> {
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

    /// CHECK: Seizure authority (PermanentDelegate) mandated by regulators
    pub permanent_delegate: UncheckedAccount<'info>,

    /// CHECK: Authority for confidential transfer configuration
    pub confidential_transfer_authority: UncheckedAccount<'info>,

    pub token_2022_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

impl<'info> ReissueRemittanceMint<'info> {
    pub fn reissue_mint(
        &mut self,
        transfer_fee_basis_points: u16,
        maximum_fee: u64,
        decimals: u8,
    ) -> Result<()> {
        require!(
            transfer_fee_basis_points <= MAX_FEE_BASIS_POINTS,
            RemittanceError::InvalidFeeRate
        );

        // Extensions correctly sized via try_calculate_account_len
        // Note: Token-2022 strictly requires ConfidentialTransferFeeConfig when combining
        // TransferFeeConfig and ConfidentialTransferMint!
        let extension_types = [
            ExtensionType::TransferFeeConfig,
            ExtensionType::MetadataPointer,
            ExtensionType::DefaultAccountState,
            ExtensionType::MintCloseAuthority,
            ExtensionType::PermanentDelegate,
            ExtensionType::ConfidentialTransferMint,
            ExtensionType::ConfidentialTransferFeeConfig,
        ];

        let space = ExtensionType::try_calculate_account_len::<Mint>(&extension_types)
            .map_err(|_| RemittanceError::InvalidExtension)?;

        let lamports = Rent::get()?.minimum_balance(space);

        // 1. Create the uninitialized account
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

        // 2. Initialize TransferFeeConfig
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

        // 3. Initialize MetadataPointer (pointed at mint itself)
        let init_metadata_pointer_ix = initialize_metadata_pointer(
            self.token_2022_program.key,
            self.mint.key,
            Some(*self.metadata_pointer_authority.key),
            Some(*self.mint.key),
        )?;
        anchor_lang::solana_program::program::invoke(
            &init_metadata_pointer_ix,
            &[self.mint.to_account_info()],
        )?;

        // 4. Initialize DefaultAccountState (Frozen)
        let init_default_state_ix = initialize_default_account_state(
            self.token_2022_program.key,
            self.mint.key,
            &AccountState::Frozen,
        )?;
        anchor_lang::solana_program::program::invoke(
            &init_default_state_ix,
            &[self.mint.to_account_info()],
        )?;

        // 5. Initialize MintCloseAuthority
        let init_close_authority_ix = initialize_mint_close_authority(
            self.token_2022_program.key,
            self.mint.key,
            Some(self.close_authority.key),
        )?;
        anchor_lang::solana_program::program::invoke(
            &init_close_authority_ix,
            &[self.mint.to_account_info()],
        )?;

        // 6. Initialize PermanentDelegate (Regulatory Seizure Authority)
        let init_perm_delegate_ix = initialize_permanent_delegate(
            self.token_2022_program.key,
            self.mint.key,
            self.permanent_delegate.key,
        )?;
        anchor_lang::solana_program::program::invoke(
            &init_perm_delegate_ix,
            &[self.mint.to_account_info()],
        )?;

        // 7. Initialize ConfidentialTransferMint with approve_policy = manual (auto_approve_new_accounts = false)
        let init_confidential_transfer_ix = initialize_confidential_transfer_mint(
            self.token_2022_program.key,
            self.mint.key,
            Some(*self.confidential_transfer_authority.key),
            false, // approve_policy = manual (auto_approve_new_accounts = false)
            None,  // Optional auditor ElGamal pubkey
        )?;
        anchor_lang::solana_program::program::invoke(
            &init_confidential_transfer_ix,
            &[self.mint.to_account_info()],
        )?;

        // 8. Initialize ConfidentialTransferFeeConfig (mandatory when combining TransferFeeConfig and ConfidentialTransferMint)
        let init_confidential_transfer_fee_ix = initialize_confidential_transfer_fee_config(
            self.token_2022_program.key,
            self.mint.key,
            Some(*self.withdraw_withheld_authority.key),
            &PodElGamalPubkey::default(),
        )?;
        anchor_lang::solana_program::program::invoke(
            &init_confidential_transfer_fee_ix,
            &[self.mint.to_account_info()],
        )?;

        // 9. InitializeMint2 (MUST be after all extension-inits!)
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
