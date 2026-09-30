use anchor_lang::prelude::*;
use anchor_spl::token_2022::Token2022;
use spl_token_2022_interface::{
    extension::confidential_transfer::{
        instruction::{
            deposit,
            inner_apply_pending_balance,
            inner_configure_account,
            inner_transfer,
            inner_withdraw,
            BatchedGroupedCiphertext3HandlesValidityProofData,
            BatchedRangeProofU128Data,
            BatchedRangeProofU64Data,
            CiphertextCommitmentEqualityProofData,
            PubkeyValidityProofData,
        },
        DecryptableBalance,
    },
    solana_zk_sdk::encryption::pod::elgamal::PodElGamalCiphertext,
    state::AccountState,
};
use spl_token_confidential_transfer_proof_extraction::instruction::ProofLocation;
use crate::{
    error::RemittanceError,
    state::StateReader,
};

// 1. ConfigureAccount (owner-only, distinct from ATA creation-by-anyone)
#[derive(Accounts)]
pub struct ConfigureConfidentialAccount<'info> {
    pub authority: Signer<'info>,

    #[account(mut)]
    /// CHECK: Token account to configure, validated via StateReader
    pub account: UncheckedAccount<'info>,

    /// CHECK: Mint account with ConfidentialTransferMint extension
    pub mint: UncheckedAccount<'info>,

    /// CHECK: Context state account for verified PubkeyValidity proof
    pub proof_context_account: UncheckedAccount<'info>,

    pub token_2022_program: Program<'info, Token2022>,
}

impl<'info> ConfigureConfidentialAccount<'info> {
    pub fn configure(
        &self,
        decryptable_zero_balance: [u8; 36],
        maximum_pending_balance_credit_counter: u64,
    ) -> Result<()> {
        let mint_data = self.mint.try_borrow_data()?;
        let mint_state = StateReader::unpack_mint(&mint_data)?;
        StateReader::get_confidential_transfer_mint(&mint_state)?;

        let acc_data = self.account.try_borrow_data()?;
        let acc_state = StateReader::unpack_account(&acc_data)?;

        // Owner-only check: distinct from ATA creation by anyone
        require_keys_eq!(acc_state.base.owner, *self.authority.key, RemittanceError::Unauthorized);
        require_keys_eq!(acc_state.base.mint, *self.mint.key, RemittanceError::InvalidMint);

        drop(mint_data);
        drop(acc_data);

        let decryptable_balance = DecryptableBalance::from(decryptable_zero_balance);
        let ix = inner_configure_account(
            self.token_2022_program.key,
            self.account.key,
            self.mint.key,
            &decryptable_balance,
            maximum_pending_balance_credit_counter,
            self.authority.key,
            &[],
            ProofLocation::<PubkeyValidityProofData>::ContextStateAccount(self.proof_context_account.key),
        )?;

        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                self.account.to_account_info(),
                self.mint.to_account_info(),
                self.proof_context_account.to_account_info(),
                self.authority.to_account_info(),
            ],
        )?;

        Ok(())
    }
}

// 1b. ApproveAccount (Confidential Transfer Authority approval for manual approve policy)
#[derive(Accounts)]
pub struct ApproveConfidentialAccount<'info> {
    pub authority: Signer<'info>,

    #[account(mut)]
    /// CHECK: Token account to approve
    pub account: UncheckedAccount<'info>,

    /// CHECK: Mint account
    pub mint: UncheckedAccount<'info>,

    pub token_2022_program: Program<'info, Token2022>,
}

impl<'info> ApproveConfidentialAccount<'info> {
    pub fn approve(&self) -> Result<()> {
        let mint_data = self.mint.try_borrow_data()?;
        let mint_state = StateReader::unpack_mint(&mint_data)?;
        let ct_mint = StateReader::get_confidential_transfer_mint(&mint_state)?;
        require!(
            Option::<Pubkey>::from(ct_mint.authority) == Some(*self.authority.key),
            RemittanceError::Unauthorized
        );
        drop(mint_data);

        let ix = spl_token_2022_interface::extension::confidential_transfer::instruction::approve_account(
            self.token_2022_program.key,
            self.account.key,
            self.mint.key,
            self.authority.key,
            &[],
        )?;

        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                self.account.to_account_info(),
                self.mint.to_account_info(),
                self.authority.to_account_info(),
            ],
        )?;

        Ok(())
    }
}

// 2. DepositConfidentialTokens

#[derive(Accounts)]
pub struct DepositConfidentialTokens<'info> {
    pub authority: Signer<'info>,

    #[account(mut)]
    /// CHECK: Token account to deposit into
    pub account: UncheckedAccount<'info>,

    /// CHECK: Mint account
    pub mint: UncheckedAccount<'info>,

    pub token_2022_program: Program<'info, Token2022>,
}

impl<'info> DepositConfidentialTokens<'info> {
    pub fn deposit(&self, amount: u64, decimals: u8) -> Result<()> {
        let mint_data = self.mint.try_borrow_data()?;
        let mint_state = StateReader::unpack_mint(&mint_data)?;
        require_eq!(mint_state.base.decimals, decimals, RemittanceError::InvalidExtension);

        let acc_data = self.account.try_borrow_data()?;
        let acc_state = StateReader::unpack_account(&acc_data)?;
        require_keys_eq!(acc_state.base.owner, *self.authority.key, RemittanceError::Unauthorized);
        require_keys_eq!(acc_state.base.mint, *self.mint.key, RemittanceError::InvalidMint);
        require!(acc_state.base.state != AccountState::Frozen, RemittanceError::AccountFrozen);

        drop(mint_data);
        drop(acc_data);

        let ix = deposit(
            self.token_2022_program.key,
            self.account.key,
            self.mint.key,
            amount,
            decimals,
            self.authority.key,
            &[],
        )?;

        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                self.account.to_account_info(),
                self.mint.to_account_info(),
                self.authority.to_account_info(),
            ],
        )?;

        Ok(())
    }
}

// 3. ApplyPendingBalance
#[derive(Accounts)]
pub struct ApplyPendingBalance<'info> {
    pub authority: Signer<'info>,

    #[account(mut)]
    /// CHECK: Token account to apply pending balance
    pub account: UncheckedAccount<'info>,

    pub token_2022_program: Program<'info, Token2022>,
}

impl<'info> ApplyPendingBalance<'info> {
    pub fn apply(
        &self,
        expected_pending_balance_credit_counter: u64,
        new_decryptable_available_balance: [u8; 36],
    ) -> Result<()> {
        let acc_data = self.account.try_borrow_data()?;
        let acc_state = StateReader::unpack_account(&acc_data)?;
        require_keys_eq!(acc_state.base.owner, *self.authority.key, RemittanceError::Unauthorized);
        drop(acc_data);

        let decryptable_balance = DecryptableBalance::from(new_decryptable_available_balance);
        let ix = inner_apply_pending_balance(
            self.token_2022_program.key,
            self.account.key,
            expected_pending_balance_credit_counter,
            &decryptable_balance,
            self.authority.key,
            &[],
        )?;

        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                self.account.to_account_info(),
                self.authority.to_account_info(),
            ],
        )?;

        Ok(())
    }
}

// 4. Confidential Transfer
#[derive(Accounts)]
pub struct ConfidentialTransfer<'info> {
    pub authority: Signer<'info>,

    #[account(mut)]
    /// CHECK: Source confidential token account
    pub from: UncheckedAccount<'info>,

    /// CHECK: Mint account
    pub mint: UncheckedAccount<'info>,

    #[account(mut)]
    /// CHECK: Destination confidential token account
    pub to: UncheckedAccount<'info>,

    /// CHECK: Context state account for equality proof
    pub equality_proof_account: UncheckedAccount<'info>,

    /// CHECK: Context state account for ciphertext validity proof
    pub validity_proof_account: UncheckedAccount<'info>,

    /// CHECK: Context state account for range proof
    pub range_proof_account: UncheckedAccount<'info>,

    pub token_2022_program: Program<'info, Token2022>,
}

impl<'info> ConfidentialTransfer<'info> {
    pub fn transfer(
        &self,
        new_source_decryptable_available_balance: [u8; 36],
        transfer_amount_auditor_ciphertext_lo: [u8; 64],
        transfer_amount_auditor_ciphertext_hi: [u8; 64],
    ) -> Result<()> {
        let from_data = self.from.try_borrow_data()?;
        let from_state = StateReader::unpack_account(&from_data)?;
        require_keys_eq!(from_state.base.owner, *self.authority.key, RemittanceError::Unauthorized);
        require_keys_eq!(from_state.base.mint, *self.mint.key, RemittanceError::InvalidMint);

        let to_data = self.to.try_borrow_data()?;
        let to_state = StateReader::unpack_account(&to_data)?;
        require_keys_eq!(to_state.base.mint, *self.mint.key, RemittanceError::InvalidMint);

        drop(from_data);
        drop(to_data);

        let decryptable_balance = DecryptableBalance::from(new_source_decryptable_available_balance);
        let auditor_lo = PodElGamalCiphertext::from(transfer_amount_auditor_ciphertext_lo);
        let auditor_hi = PodElGamalCiphertext::from(transfer_amount_auditor_ciphertext_hi);

        let ix = inner_transfer(
            self.token_2022_program.key,
            self.from.key,
            self.mint.key,
            self.to.key,
            &decryptable_balance,
            &auditor_lo,
            &auditor_hi,
            self.authority.key,
            &[],
            ProofLocation::<CiphertextCommitmentEqualityProofData>::ContextStateAccount(self.equality_proof_account.key),
            ProofLocation::<BatchedGroupedCiphertext3HandlesValidityProofData>::ContextStateAccount(self.validity_proof_account.key),
            ProofLocation::<BatchedRangeProofU128Data>::ContextStateAccount(self.range_proof_account.key),
        )?;

        anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                self.from.to_account_info(),
                self.mint.to_account_info(),
                self.to.to_account_info(),
                self.equality_proof_account.to_account_info(),
                self.validity_proof_account.to_account_info(),
                self.range_proof_account.to_account_info(),
                self.authority.to_account_info(),
            ],
        )?;

        Ok(())
    }
}

// 5. WithdrawConfidentialTokens (including applying pending balance before withdrawal)
#[derive(Accounts)]
pub struct WithdrawConfidentialTokens<'info> {
    pub authority: Signer<'info>,

    #[account(mut)]
    /// CHECK: Confidential token account to withdraw from
    pub account: UncheckedAccount<'info>,

    /// CHECK: Mint account
    pub mint: UncheckedAccount<'info>,

    /// CHECK: Context state account for equality proof
    pub equality_proof_account: UncheckedAccount<'info>,

    /// CHECK: Context state account for range proof
    pub range_proof_account: UncheckedAccount<'info>,

    pub token_2022_program: Program<'info, Token2022>,
}

impl<'info> WithdrawConfidentialTokens<'info> {
    pub fn withdraw(
        &self,
        amount: u64,
        decimals: u8,
        new_decryptable_available_balance: [u8; 36],
        apply_pending_balance_first: bool,
        expected_pending_balance_credit_counter: u64,
        intermediate_decryptable_available_balance: Option<[u8; 36]>,
    ) -> Result<()> {
        let acc_data = self.account.try_borrow_data()?;
        let acc_state = StateReader::unpack_account(&acc_data)?;
        require_keys_eq!(acc_state.base.owner, *self.authority.key, RemittanceError::Unauthorized);
        require_keys_eq!(acc_state.base.mint, *self.mint.key, RemittanceError::InvalidMint);
        drop(acc_data);

        // Crucial requirement: Applying pending balance before withdrawal
        if apply_pending_balance_first {
            let intermediate_balance = intermediate_decryptable_available_balance
                .map(DecryptableBalance::from)
                .unwrap_or_else(|| DecryptableBalance::from(new_decryptable_available_balance));

            let apply_ix = inner_apply_pending_balance(
                self.token_2022_program.key,
                self.account.key,
                expected_pending_balance_credit_counter,
                &intermediate_balance,
                self.authority.key,
                &[],
            )?;

            anchor_lang::solana_program::program::invoke(
                &apply_ix,
                &[
                    self.account.to_account_info(),
                    self.authority.to_account_info(),
                ],
            )?;
        }

        let decryptable_balance = DecryptableBalance::from(new_decryptable_available_balance);
        let withdraw_ix = inner_withdraw(
            self.token_2022_program.key,
            self.account.key,
            self.mint.key,
            amount,
            decimals,
            &decryptable_balance,
            self.authority.key,
            &[],
            ProofLocation::<CiphertextCommitmentEqualityProofData>::ContextStateAccount(self.equality_proof_account.key),
            ProofLocation::<BatchedRangeProofU64Data>::ContextStateAccount(self.range_proof_account.key),
        )?;

        anchor_lang::solana_program::program::invoke(
            &withdraw_ix,
            &[
                self.account.to_account_info(),
                self.mint.to_account_info(),
                self.equality_proof_account.to_account_info(),
                self.range_proof_account.to_account_info(),
                self.authority.to_account_info(),
            ],
        )?;

        Ok(())
    }
}
